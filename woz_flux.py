import math
import bitarray
import matplotlib.pyplot as plt

signal_chunks = []
SEQ_ROM = [
    0x18, 0x18, 0x18, 0x18, 0x0A, 0x0A, 0x0A, 0x0A, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18,
	0x2D, 0x38, 0x2D, 0x38, 0x0A, 0x0A, 0x0A, 0x0A, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28,
	0x38, 0x28, 0xD8, 0x08, 0x0A, 0x0A, 0x0A, 0x0A, 0x39, 0x39, 0x39, 0x39, 0x3B, 0x3B, 0x3B, 0x3B,
	0x48, 0x48, 0xD8, 0x48, 0x0A, 0x0A, 0x0A, 0x0A, 0x48, 0x48, 0x48, 0x48, 0x48, 0x48, 0x48, 0x48,
	0x58, 0x58, 0xD8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58,
	0x68, 0x68, 0xD8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x68, 0x68, 0x68, 0x68, 0x68, 0x68, 0x68, 0x68,
	0x78, 0x78, 0xD8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78,
	0x88, 0x88, 0xD8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x08, 0x88, 0x08, 0x88, 0x08, 0x88, 0x08, 0x88,
	0x98, 0x98, 0xD8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x98, 0x98, 0x98, 0x98, 0x98, 0x98, 0x98, 0x98,
	0x29, 0xA8, 0xD8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8,
	0xBD, 0xB8, 0xCD, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xB9, 0xB9, 0xB9, 0xB9, 0xBB, 0xBB, 0xBB, 0xBB,
	0x59, 0xC8, 0xD9, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8,
	0xD9, 0xA0, 0xD9, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8,
	0x08, 0xE8, 0xD8, 0xE8, 0x0A, 0x0A, 0x0A, 0x0A, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8,
	0xFD, 0xF8, 0xFD, 0xF8, 0x0A, 0x0A, 0x0A, 0x0A, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8,
	0x4D, 0xE0, 0xDD, 0xE0, 0x0A, 0x0A, 0x0A, 0x0A, 0x88, 0x08, 0x88, 0x08, 0x88, 0x08, 0x88, 0x08]

SEQ_MNEMONICS = ['CLR', None,  None, None, None, None,  None, None,
                 'NOP', 'SL0', 'SR', 'LD', None, 'SL1', None, None]

class Sequencer:
    def __init__(self):
        self.data_register = 0
        self._state = 2 << 4
        #self._one_us_in_ticks = one_us_in_ticks
        #self._delay = 0

    def seq_rom_value(self, state, rwSwitch, slSwitch, readPulse, qa):
        r = (state & 0xF0) | (rwSwitch << 3) | (slSwitch << 2) | (readPulse << 1) | qa
        return SEQ_ROM[r]
        
    def _next_state(self, rwSwitch, slSwitch, readPulse):  
        # Sather 9-44
        #   A1- QA, the MSB of the data register
        #   A2- SHIFT/LOAD, the $C08C,X/$C08D,X switch
        #   A3- READ/WRITE, the $CO8E,X/$C08F,X switch
        #   A4- The read pulse from the disk drive
        
        qa = (self.data_register & 0x80) >> 7
        self._state = self.seq_rom_value(self._state, rwSwitch, slSwitch, readPulse, qa)
        
    def _apply_command(self):
        mnemo = SEQ_MNEMONICS[self._state & 0xF]
        if mnemo == 'SL1':
            self.data_register = (self.data_register << 1) + 1
        elif mnemo == 'SL0':
            self.data_register = (self.data_register << 1) + 0
        elif mnemo == 'CLR':
            self.data_register = 0
        elif mnemo == 'NOP':            
            pass
        else:
            raise Exception("{mnemo} not supported")
    
    def tick(self, rw_switch, sl_switch, read_pulse):
        #if self._delay > 0:
        #    self._delay -= 1
        #else:
        #    self._delay = 0.5 * self._one_us_in_ticks
        self._next_state(rw_switch, sl_switch, read_pulse)
        self._apply_command()

def plot_mc3470(xlim, title):
    from matplotlib.ticker import (AutoMinorLocator, MultipleLocator)
    # clip
    for i in range(len(signal_chunks)):
        start_time, type_, duration = signal_chunks[i+1]
        if start_time + duration > xlim[0]:
            break
    j = i
    while True:
        start_time, type_, duration = signal_chunks[j]
        if start_time < xlim[1]:
            j += 1
        else:
            break
            
    start_chunk=i
    end_chunk=j
    start = signal_chunks[start_chunk][0]
    end = signal_chunks[end_chunk][0]
            
    f, ax = plt.subplots(1, 1, figsize = (20, 3))


    if (end-start)/8 < 40:
        tick_div = 8
    else:
        tick_div = round((end-start) / 20)
            
    #ticks = range(start,end,tick_div)
    #ax.set_xticks(ticks)
    
    #ax.set_xticklabels([f"{round(t*0.125)}" for t in ticks])
    ax.grid(which="minor", color="#ACE6D7")
    ax.grid(which="major", color="#ACE6D7")

    label_weak, label_pulse, label_no_signal = "Weak bits", "Read Pulse", "No signal"
    for chunk in signal_chunks[start_chunk:end_chunk+1]:
        start_time, type_, duration = chunk
        if type_ == NO_PULSE:
            ax.plot([start_time, start_time+duration],
                    [0,0],color="green",label=label_no_signal)
            label_no_signal = None
        elif type_ == READ_PULSE:
            ax.plot([start_time, start_time, start_time+duration, start_time+duration],
                    [0,1,1,0],color="red",label=label_pulse)
            label_pulse = None
    ax.scatter(range(0,end,4),[0.5]*len(range(0,end,4)),color="black",label="LSS read")
    ax.set_xlabel("Time (125 nanosec)")
    ax.xaxis.set_minor_locator(MultipleLocator(4))    
    ax.set_title(f"MC3470 Simulation - {title}")
    ax.tick_params(left=False, labelleft=False)
    ax.set_xlim(xlim[0],xlim[1])    
    ax.legend(loc='lower right')
    return ax


with open('minotaur.bin', 'rb') as fp:
#with open('test.bin', 'rb') as fp:
    data = fp.read()

print("File len = ",len(data))

ones = 1

for i in data:
    ones += 1

print("Number of ones = ",ones)

bs = bitarray.bitarray(endian="big")
count = 0
READ_PULSE = "RP"
NO_PULSE = "NO_SIG"
time_offset =0

for b in data:
    if b < 255:
        count += b
        bs.extend([0]*(count-1))
        bs.append(1)

        signal_chunks.append((time_offset, READ_PULSE, 8))
        signal_chunks.append((time_offset+8, NO_PULSE, count - 8))
        time_offset += count
        count = 0
    else:
        count += 255

#print(bs)

plot_mc3470([0,200],"minotaur")
#plt.show()

lss = Sequencer()
mc3470_time = 0
mc3470_chunk = 0
lss_time = 0
data_register = []
read_pulses = []

while True:
    lss_end_time = lss_time + 4 - 1 # 4*0.125=0.5 microsec
    if lss_end_time > 1280:
        break
    
    read_pulse = 0
    start_time, type_, duration = signal_chunks[mc3470_chunk]   
    chunk_end_time = start_time + duration - 1
    while True:
        if type_ == NO_PULSE:
            pass
        elif type_ == READ_PULSE:
            read_pulse |= 1
            
        mc3470_time += 1 # Can be 0.95, 1.05, the real speed...
        
        if   mc3470_time <= chunk_end_time  and mc3470_time <= lss_end_time:
            pass
        elif mc3470_time <= chunk_end_time  and mc3470_time > lss_end_time:
            break
        elif mc3470_time > chunk_end_time and mc3470_time <= lss_end_time:
            mc3470_chunk += 1
            start_time, type_, duration = signal_chunks[mc3470_chunk]   
            chunk_end_time = start_time + duration - 1
        elif mc3470_time > chunk_end_time and mc3470_time > lss_end_time:
            mc3470_chunk += 1
            start_time, type_, duration = signal_chunks[mc3470_chunk]   
            chunk_end_time = start_time + duration - 1
            break
            
    read_pulses.append(read_pulse)

    print(read_pulse,end="")

    lss.tick(0,0,read_pulse)
    data_register.append(lss.data_register)
    lss_time += 4

print()
print(f"LSS time:{lss_time}, MC3470 time:{mc3470_time}, chunk:{mc3470_chunk}")    

filtered = [0]
watch = False
for dr in data_register:
    if dr & 0x80 != 0 and not watch:
        filtered.append(dr)
        watch = True
    elif dr & 0x80 == 0:
        watch = False
print(",".join([f"0x{dr:02X}" for dr in filtered]))



