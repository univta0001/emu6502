fn setcmd(lssrom: &mut [u8], x: usize, cmd: u8) {
    lssrom[x] = (lssrom[x] & 0xf0) | cmd
}

fn setseq(lssrom: &mut [u8], x: usize, cmd: u8) {
    lssrom[x] = (lssrom[x] & 0x0f) | cmd
}

fn setbth(lssrom: &mut [u8], x: usize, cmd: u8) {
    lssrom[x] = cmd
}

fn main() {
    let mut lssrom:Vec<u8> = vec![0u8;256];
    for s in 0 .. 0x10 {
        let seq = s << 4;
        for adr in 0 .. 0x10 {
            lssrom[seq|adr] = (seq+0x18 as usize) as u8; 
            if adr & 0xc == 4 {
                lssrom[seq|adr] = 0x0a;
            }
            if adr == 1 || adr == 3 {
                lssrom[seq|adr] = 0xd8;
            }
        }
    }

    setcmd(&mut lssrom,0x10,0x0d);
    setbth(&mut lssrom,0x90,0x29);
    setcmd(&mut lssrom,0xa0,0x0d);
    setbth(&mut lssrom,0xb0,0x59);
    setcmd(&mut lssrom,0xc0,0x09);
    setseq(&mut lssrom,0xd0,0x00);
    setcmd(&mut lssrom,0xe0,0x0d);
    setbth(&mut lssrom,0xf0,0x4d);

    setseq(&mut lssrom,0x01,0x10);
    setbth(&mut lssrom,0x11,0x2d);
    setbth(&mut lssrom,0xa1,0xcd);
    setcmd(&mut lssrom,0xb1,0x09);
    setcmd(&mut lssrom,0xc1,0x09);
    setbth(&mut lssrom,0xe1,0xfd);
    setcmd(&mut lssrom,0xf1,0x0d);

    setseq(&mut lssrom,0x12,0x30);
    setseq(&mut lssrom,0x22,0x20);
    setbth(&mut lssrom,0xc2,0xa0);
    setbth(&mut lssrom,0xf2,0xe0);

    setseq(&mut lssrom,0x03,0x10);
    setseq(&mut lssrom,0x13,0x30);
    setseq(&mut lssrom,0x23,0x00);
    setseq(&mut lssrom,0x33,0x40);
    setseq(&mut lssrom,0xd3,0xe0);
    setseq(&mut lssrom,0xe3,0xf0);
    setbth(&mut lssrom,0xf3,0xe0);

    setcmd(&mut lssrom,0x28,0x09);
    setseq(&mut lssrom,0x78,0x00);
    setcmd(&mut lssrom,0xa8,0x09);
    setseq(&mut lssrom,0xf8,0x80);

    setcmd(&mut lssrom,0x29,0x09);
    setseq(&mut lssrom,0x79,0x00);
    setcmd(&mut lssrom,0xa9,0x09);
    setseq(&mut lssrom,0xf9,0x80);

    setcmd(&mut lssrom,0x2a,0x09);
    setcmd(&mut lssrom,0xaa,0x09);

    setcmd(&mut lssrom,0x2b,0x09);
    setcmd(&mut lssrom,0xab,0x09);

    setcmd(&mut lssrom,0x2c,0x0b);
    setseq(&mut lssrom,0x7c,0x00);
    setcmd(&mut lssrom,0xac,0x0b);
    setseq(&mut lssrom,0xfc,0x80);

    setcmd(&mut lssrom,0x2d,0x0b);
    setseq(&mut lssrom,0x7d,0x00);
    setcmd(&mut lssrom,0xad,0x0b);
    setseq(&mut lssrom,0xfd,0x80);

    setcmd(&mut lssrom,0x2e,0x0b);
    setcmd(&mut lssrom,0xae,0x0b);

    setcmd(&mut lssrom,0x2f,0x0b);
    setcmd(&mut lssrom,0xaf,0x0b);
    
    for i in (0..256).step_by(16) {
        eprint!("     ");
        for j in 0..16 {
            eprint!("0x{:02X}, ", lssrom[(i+j) as usize]);
        }
        eprintln!("// {:X}",i/16);
    }    
 
}
