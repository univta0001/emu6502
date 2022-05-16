import java.awt.image.BufferedImage;
import javax.imageio.ImageIO;
import java.io.*;

public class test_dhr2{

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
        new test_dhr2().start(args);
    }

    public int getBit(int value) {
        if ((value & 0xff) != 0) {
            return 1;
        } else {
            return 0;
        }
    }

    public void start(String args[]) throws Exception {
        System.out.println("Loading "+args[0]);
        BufferedImage bi = ImageIO.read(new File(args[0]));
        BufferedImage bout = new BufferedImage(bi.getWidth(), bi.getHeight(), 
            BufferedImage.TYPE_INT_RGB);

        System.out.println("Image width  = "+bi.getWidth());
        System.out.println("Image height = "+bi.getHeight());

        boolean remove_artifact = true;

        if (args.length > 1) {
            remove_artifact = Boolean.valueOf(args[1]);
        }

        int prev_index = 0;
        for (int yy = 0; yy<bi.getHeight(); yy++) {
            for (int xx = 0; xx<bi.getWidth(); xx +=4) {
                int color_index = 
                    (getBit(bi.getRGB(xx+3,yy)) << 3) +
                    (getBit(bi.getRGB(xx+2,yy)) << 2) +
                    (getBit(bi.getRGB(xx+1,yy)) << 1) + getBit(bi.getRGB(xx,yy));

                bout.setRGB(xx,yy, colors[color_index]);
                bout.setRGB(xx+1,yy, colors[color_index]);
                bout.setRGB(xx+2,yy, colors[color_index]);
                bout.setRGB(xx+3,yy, colors[color_index]);

                if (remove_artifact) {
                    // Handling White (Case 0111 1000)
                    if ((color_index & 7) == 1 &&  ((prev_index & 0xf) == 14)) {
                        if (xx-3 >= 0) {
                            bout.setRGB(xx-3,yy, colors[15]);
                        }
                        if (xx-2 >= 0) {
                            bout.setRGB(xx-2,yy, colors[15]);
                        }
                        if (xx-1 >= 0) {
                            bout.setRGB(xx-1,yy, colors[15]);
                        }
                        bout.setRGB(xx,yy, colors[15]);
                    }

                    // Handling White (Case 0011 1100)
                    if ((color_index & 3) == 3 &&  (prev_index >> 2 == (color_index & 3))) {
                        if ((prev_index & 3) == 0) {
                            if (xx-2 >=0) {
                                bout.setRGB(xx-2,yy, colors[15]);
                            }
                            if (xx-1 >=0) {
                                bout.setRGB(xx-1,yy, colors[15]);
                            }
                            bout.setRGB(xx,yy, colors[15]);
                            if (xx+1 < 560) {
                                bout.setRGB(xx+1,yy, colors[15]);
                            }
                        }
                    }

                    // Handling White (Case 0001 1110)
                    if ((color_index & 7) == 7 &&  ((prev_index & 0x8) != 0)) {
                        if (xx-1 >=0) {
                            bout.setRGB(xx-1,yy, colors[15]);
                        }
                        bout.setRGB(xx,yy, colors[15]);
                        if (xx+1 < 560) {
                            bout.setRGB(xx+1,yy, colors[15]);
                        }
                        if (xx+2 < 560) {
                            bout.setRGB(xx+2,yy, colors[15]);
                        }
                    }     

                    // Handling Black (Case x000 0yyy)
                    if ((color_index & 1) == 0 &&  (prev_index & 0xe) == 0) {
                        if (xx-3 >= 0) {
                            bout.setRGB(xx-3,yy, colors[0]);
                        }
                        if (xx-2 >= 0) {
                            bout.setRGB(xx-2,yy, colors[0]);
                        }
                        if (xx-1 >= 0) {
                            bout.setRGB(xx-1,yy, colors[0]);
                        }
                        bout.setRGB(xx,yy, colors[0]);
                    } 

                    // Handling Black (Case xx00 00yy)
                    if ((color_index & 3) == 0 && ((prev_index & 0xc) == 0)) {
                        if (xx-2 >= 0) {
                            bout.setRGB(xx-2,yy, colors[0]);
                        }
                        if (xx-1 >= 0) {
                            bout.setRGB(xx-1,yy, colors[0]);
                        }
                        bout.setRGB(xx,yy, colors[0]);
                        if (xx+1 < 560) {
                            bout.setRGB(xx+1,yy, colors[0]);
                        }
                    }

                    // Handling Black (Case xxx0 000y)
                    if ((color_index & 7) == 0 &&  ((prev_index & 0x8) == 0)) {
                        if (xx-1 >=0) {
                            bout.setRGB(xx-1,yy, colors[0]);
                        }
                        bout.setRGB(xx,yy, colors[0]);
                        if (xx+1 < 560) {
                            bout.setRGB(xx+1,yy, colors[0]);
                        }
                        if (xx+2 < 560) {
                            bout.setRGB(xx+2,yy, colors[0]);
                        }
                    } 
                }

                prev_index = color_index;
            }
        }
        
        ImageIO.write(bout, "PNG", new File("test.png"));
    }

}
