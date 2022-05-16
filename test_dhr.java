import java.awt.image.BufferedImage;
import javax.imageio.ImageIO;
import java.io.*;

public class test_dhr{

    int[] colors = new int[] {
        0,
        0x21*65536+0x1d*256+0xff,
        0x00*65536+0x92*256+0x09,
        0x00*65536+0xb0*256+0xff,
        0x5d*65536+0x61*256+0x00,
        0x7f*65536+0x7f*256+0x7f,
        0x0f*65536+0xf4*256+0x00,
        0x31*65536+0xff*256+0x89,
        0xcd*65536+0x00*256+0x75,
        0xef*65536+0x0a*256+0xff,
        0x7f*65536+0x7f*256+0x7f,
        0x0a*65536+0x9d*256+0xff,
        0xff*65536+0x4e*256+0x00,
        0xff*65536+0x6c*256+0xf5,
        0xdd*65536+0xe1*256+0x00,
        0xff*65536+0xff*256+0xff
    };

    public static void main(String args[]) throws Exception {
        new test_dhr().start(args);
    }

    public int get_base(int row) {
        int ab = row & 0xc0;
        int e = (row & 0x8) << 4;
        int cd = (row & 0x30) << 4;
        int fgh = (row & 0x7) << 10;
        int addr = fgh | cd | e | ab >> 1 | ab >> 3;
        return addr;
    }
    
    public void start(String args[]) throws Exception {

        System.out.println("Loading "+args[0]);
        File file = new File(args[0]);
        FileInputStream fis = new FileInputStream(file);
        byte[] data = new byte[(int) file.length()];
        fis.read(data);
        fis.close();

        BufferedImage bi = new BufferedImage(560, 384, BufferedImage.TYPE_INT_RGB);

        for (int j = 0; j < 384; j++) {
            for (int i = 0; i < 560; i++) {
                bi.setRGB(i,j, colors[15]);
            }
        }

        for (int j = 0; j < 192; j++) {
            int base = get_base(j);

            for (int i = 0 ; i < 40; i++) {
                int x = i*14;

                //
                //      Col 0             Col 1
                //   Aux      Main     Aux      Main
                // 76543210 76543210 76543210 76543210
                //  bbbaaaa  ddccccb  feeeedd  ggggfff
                //
                //
                // byte1 + (byte2&0x7f) << 7 + (byte3 & 0x7f) << 14 + (byte4 & 0x7f) << 21;
                //
                
                int ptr = i-i%2;
                
                int value_7_pixels = (data[8192+base+ptr] & 0x7f) + ((data[base+ptr] & 0x7f) << 7);
                if (ptr+1 < 40) {
                    value_7_pixels += ((data[8192+base+ptr+1] & 0x7f) << 14) +
                        ((data[base+ptr+1] & 0x7f) << 21);
                }

                if (i%2 == 0) 
                {
                    int k = 0;
                    for (int l = 0; l<3; l++) {
                        int color = value_7_pixels & 0xf;
                        bi.setRGB(x+k,j*2, colors[color]);
                        bi.setRGB(x+k,j*2+1, colors[color]);
                        bi.setRGB(x+k+1,j*2, colors[color]);
                        bi.setRGB(x+k+1,j*2+1, colors[color]);
                        bi.setRGB(x+k+2,j*2, colors[color]);
                        bi.setRGB(x+k+2,j*2+1, colors[color]);
                        bi.setRGB(x+k+3,j*2, colors[color]);
                        bi.setRGB(x+k+3,j*2+1, colors[color]);
                        k += 4;
                        value_7_pixels >>= 4;
                    }
                    int color = value_7_pixels & 0xf;
                    bi.setRGB(x+k,j*2, colors[color]);
                    bi.setRGB(x+k,j*2+1, colors[color]);
                    bi.setRGB(x+k+1,j*2, colors[color]);
                    bi.setRGB(x+k+1,j*2+1, colors[color]);

                } else {
                    value_7_pixels >>= 12;
                    int color = value_7_pixels & 0xf;
                    int k=0;
                    bi.setRGB(x+k,j*2, colors[color]);
                    bi.setRGB(x+k,j*2+1, colors[color]);
                    bi.setRGB(x+k+1,j*2, colors[color]);
                    bi.setRGB(x+k+1,j*2+1, colors[color]);
                    value_7_pixels >>= 4;
                    k += 2;
                    for (int l=0; l<3; l++) {
                        color = value_7_pixels & 0xf;
                        bi.setRGB(x+k,j*2, colors[color]);
                        bi.setRGB(x+k,j*2+1, colors[color]);
                        bi.setRGB(x+k+1,j*2, colors[color]);
                        bi.setRGB(x+k+1,j*2+1, colors[color]);
                        bi.setRGB(x+k+2,j*2, colors[color]);
                        bi.setRGB(x+k+2,j*2+1, colors[color]);
                        bi.setRGB(x+k+3,j*2, colors[color]);
                        bi.setRGB(x+k+3,j*2+1, colors[color]);
                        
                        k += 4;
                        value_7_pixels >>= 4;
                    }
                }
            }
        }
        ImageIO.write(bi, "PNG", new File("test.png"));
    }

}
