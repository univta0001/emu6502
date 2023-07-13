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

        int color_index = 0;
        
        for (int yy = 0; yy<bi.getHeight(); yy++) {
            color_index = 0;
            for (int xx = 0; xx<bi.getWidth(); xx += 1) {
                int index = xx % 4;
                int bit = getBit(bi.getRGB(xx,yy));
                if (bit > 0) {
                    color_index = color_index | (1 << index);
                } else {
                    color_index = color_index & ((1 << index) ^ 0xf);
                }
                bout.setRGB(xx,yy, colors[color_index]);
            }
        }
        
        ImageIO.write(bout, "PNG", new File("test.png"));
    }

}
