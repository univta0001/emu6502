import java.awt.image.BufferedImage;
import javax.imageio.ImageIO;
import java.io.*;

public class test_ntsc{

    // YIQ to RGB
    // R = Y + 0.956 * I + 0.619 * Q
    // G = Y - 0.272 * I - 0.647 * Q
    // B = Y - 1.106 * I + 1.703 * Q
    //
    // YUV to RGB
    // R = Y + 0.00000 * U + 1.13983 * V
    // G = Y - 0.39465 * U - 0.58060 * V
    // B = Y + 2.03211 * U + 0.00000 * V
    //
    // Color Name  GR 	HGR DHGR Chroma Phase Chroma Amplitude Luma 	R 	G 	B
    // Black 	   0 	0,4 0 	 0 	          0 	           0 	    0 	0 	0
    // Gray 	   5 	n/a 5 	 0 	          0 	           50 	    156 156 156
    // Grey 	   10 	n/a 10 	 0 	          0 	           50 	    156 156 156
    // White 	   15 	3,7 15 	 0 	          0 	           100 	    255 255 255
    // dk Blue 	   2 	n/a	8 	 0 	          60 	           25 	    96 	78 	189
    // lt Blue 	   7 	n/a	13 	 0 	          60 	           75 	    208 195 255
    // Purple 	   3 	2 	9 	 45 	      100 	           50 	    255 68 	253
    // dk Red 	   1 	n/a	1 	 90 	      60 	           25 	    227 30 	96
    // Pink 	   11 	n/a	11 	 90 	      60 	           75 	    255 160 208
    // Orange 	   9 	5   3 	 135 	      100              50 	    255 106 60
    // Brown 	   8 	n/a	2 	 180 	      60 	           25 	    96 	114 3
    // Yellow 	   13 	n/a	7 	 180 	      60 	           75 	    208 221 141
    // lt Green    12 	1   6 	 225 	      100              50 	    20 	245 60
    // dk Green    4 	n/a 4 	 270 	      60 	           25 	    0 	163 96
    // Aqua 	   14 	n/a	14 	 270 	      60 	           75 	    114 255 208
    // med Blue    6 	6   12 	 315 	      100              50 	    20 	207 253


    static double [] wy = new double[] { 0.0012969893416029652, -0.0007181026579346038, -0.009993295243617743, -0.022103770406847224, -0.015383159829078933, 0.03665489905357039, 0.1346162732936952, 0.23592607033063934, 0.27940819223594177, 0.23592607033063925, 0.1346162732936951, 0.036654899053570264, -0.01538315982907891, -0.022103770406847304, -0.009993295243617743, -0.0007181026579346008, 0.0012969893416029652 };

    static double [] wu = new double[] { 0.005147919379870954, 0.015432708160017513, 0.03647844231774774, 0.07008157238459167, 0.1150901077856405, 0.16601592153114964, 0.2137255023867992, 0.2478785632740175, 0.26029852556033184, 0.24787856327401742, 0.21372550238679902, 0.1660159215311491, 0.11509010778564033, 0.07008157238459192, 0.03647844231774774, 0.015432708160017447, 0.005147919379870954 };

    static double [] wv = new double[] { 0.005147919379870954, 0.015432708160017513, 0.03647844231774774, 0.07008157238459167, 0.1150901077856405, 0.16601592153114964, 0.2137255023867992, 0.2478785632740175, 0.26029852556033184, 0.24787856327401742, 0.21372550238679902, 0.1660159215311491, 0.11509010778564033, 0.07008157238459192, 0.03647844231774774, 0.015432708160017447, 0.005147919379870954 };

    static double[][] cs = new double[][] {
        new double[] { wy[8], wu[8], wv[8] },
        new double[] { wy[7], wu[7], wv[7] },
        new double[] { wy[6], wu[6], wv[6] },
        new double[] { wy[5], wu[5], wv[5] },
        new double[] { wy[4], wu[4], wv[4] },
        new double[] { wy[3], wu[3], wv[3] },
        new double[] { wy[2], wu[2], wv[2] },
        new double[] { wy[1], wu[1], wv[1] },
        new double[] { wy[0], wu[0], wv[0] }
    };

    public static void main(String args[]) throws Exception {
        new test_ntsc().start(args);
    }

    public double[] mul(double[] color, double[] w) {
        double[] result = new double[color.length];
        for (int i = 0; i < color.length; i++) {
            result[i] = color[i] * w[i];
        }
        return result;
    }

    public double[] add(double[] color, double[] w) {
        return new double[] { color[0]+w[0], color[1]+w[1], color[2]+w[2] };
    }    

    public double[] pixel_at(double[] yuv_image, int x, int y) {
        if (x < 0 || y < 0 || x >= 560 || y >= 384) {
            return new double[] { 0, 0, 0 };
        }
        int index = (x + y * 560)*3;

        return new double[] { yuv_image[index], yuv_image[index+1], yuv_image[index+2] };
    }

    public double[] pixels(double[] yuv_image, int x, int y, int i) {
        double[] l_pixel = pixel_at(yuv_image,x-i,y);
        double[] r_pixel = pixel_at(yuv_image,x+i,y);

        return new double[] { l_pixel[0] + r_pixel[0], l_pixel[1] + r_pixel[1],
            l_pixel[2] + r_pixel[2] };
    }

    public double[] color_at(double[] yuv_image, int x, int y) {
        double [] c = mul(pixel_at(yuv_image,x,y), cs[0]);
        c = add(c, mul(pixels(yuv_image,x,y,1), cs[1]));
        c = add(c, mul(pixels(yuv_image,x,y,2), cs[2]));
        c = add(c, mul(pixels(yuv_image,x,y,3), cs[3]));
        c = add(c, mul(pixels(yuv_image,x,y,4), cs[4]));
        c = add(c, mul(pixels(yuv_image,x,y,5), cs[5]));
        c = add(c, mul(pixels(yuv_image,x,y,6), cs[6]));
        c = add(c, mul(pixels(yuv_image,x,y,7), cs[7]));
        c = add(c, mul(pixels(yuv_image,x,y,8), cs[8]));
        return c;
    }

    public double acosh(double x) {
        return Math.log(x + Math.sqrt(x*x - 1.0));
    }

    public double[] realIDFT(double[] array) {
        int size = array.length;
        double [] w = new double[size];
        for (int i = 0;i<size;i++) {
            double omega = 2*Math.PI * i / size;
            for (int j = 0; j<size; j++) {
                w[i] += array[j] * Math.cos(j*omega);
            }
            w[i] /= 1.0/size;
        }
        return w;
    }

    public double[] chebyshevWindow(int n, double slidelobeDb) {
        int m = n-1;
        double [] w = new double[m];
        double alpha = Math.cosh(acosh(Math.pow(10, slidelobeDb / 20)) / m);
        for (int i = 0;i<m;i++) {
            double a = Math.abs(alpha * Math.cos(Math.PI * i / m));
            if (a > 1) {
                w[i] = Math.pow(-1,i)*Math.cosh(m*acosh(a));
            } else {
                w[i] = Math.pow(-1,i)*Math.cos(m*Math.acos(a));
            }
        }

        w = realIDFT(w);
        double [] t = new double[n];
        for (int i = 0; i<Math.min(n,w.length); i++) {
            t[i] = w[i];
        }
        w = t;
        w[0] /= 2;
        w[n-1] = w[0];

        double max = 0.0;
        for (int i = 0; i<n; i++) {
            if (Math.abs(w[i]) > max) {
                max = Math.abs(w[i]);
            }
        }

        for (int i = 0; i<n; i++) {
            w[i] /= (1.0/max);
        }

        return w;
    }

    public double[] lanczosWindow(int n, double fc) {
        double[] v = new double[n];
        fc = Math.min(fc,0.5);
        double halfN = Math.floor(n/2);
        for (int i =0;i<n;i++) {
            double x = 2*Math.PI * fc * (i-halfN);
            v[i] = (x == 0.0) ? 1.0 : Math.sin(x) / x;
        }
        return v;
    }

    public double[] normalize(double[] array) {
        double sum = 0.0;
        for (int i = 0;i<array.length;i++) {
            sum += array[i];
        }
        for (int i = 0;i<array.length;i++) {
            array[i] /= sum;
        }
        return array;
    }

    public double[] scale(double[] array, double value) {
        for (int i = 0;i<array.length;i++) {
            array[i] *= value;
        }
        return array;
    }

    public void start(String args[]) throws Exception {
        System.out.println("Loading "+args[0]);
        BufferedImage bi = ImageIO.read(new File(args[0]));
        BufferedImage bout = new BufferedImage(bi.getWidth(), bi.getHeight(), 
            BufferedImage.TYPE_INT_RGB);

        System.out.println("Image width  = "+bi.getWidth());
        System.out.println("Image height = "+bi.getHeight());

        boolean dhgr = true;

        if (args.length > 1) {
            dhgr = Boolean.valueOf(args[1]);
        }

        double sample_rate = 14318181.818181818;
        double subcarrier = 0.25;
        double luma_bandwidth = 2000000;
        double chroma_bandwidth = 600000;
        double y_bandwidth = luma_bandwidth / sample_rate;
        double u_bandwidth = chroma_bandwidth / sample_rate;
        double v_bandwidth = u_bandwidth;

        // Normalize chebyshevWindow
        double temp_w[] = normalize(chebyshevWindow(17,50));
        double temp_wy[] = normalize(mul(temp_w, lanczosWindow(17, y_bandwidth)));
        double temp_wu[] = scale(normalize(mul(temp_w, lanczosWindow(17, u_bandwidth))),2);
        double temp_wv[] = scale(normalize(mul(temp_w, lanczosWindow(17, v_bandwidth))),2);

        cs = new double[][] {
            new double[] { temp_wy[8], temp_wu[8], temp_wv[8] },
            new double[] { temp_wy[7], temp_wu[7], temp_wv[7] },
            new double[] { temp_wy[6], temp_wu[6], temp_wv[6] },
            new double[] { temp_wy[5], temp_wu[5], temp_wv[5] },
            new double[] { temp_wy[4], temp_wu[4], temp_wv[4] },
            new double[] { temp_wy[3], temp_wu[3], temp_wv[3] },
            new double[] { temp_wy[2], temp_wu[2], temp_wv[2] },
            new double[] { temp_wy[1], temp_wu[1], temp_wv[1] },
            new double[] { temp_wy[0], temp_wu[0], temp_wv[0] }
        };

        int[] raw_image = new int[4*bi.getWidth()*bi.getHeight()];
        double[] yuv_image = new double[3*bi.getWidth()*bi.getHeight()];

        int i = 0;
        int j = 0;
        for (int yy = 0; yy<bi.getHeight(); yy++) {
            for (int xx = 0; xx<bi.getWidth(); xx++) {
                int index = (xx+yy*bi.getWidth())*4;
                int rgb_value = bi.getRGB(xx,yy);
                raw_image[i] = rgb_value >>> 16 & 0xff;
                raw_image[i+1] = rgb_value >>> 8 & 0xff;
                raw_image[i+2] = rgb_value & 0xff;

                double p = 0.9083333333333333;

                // Double hires
                double phase = 2.0 * Math.PI * (subcarrier * (xx + 77 + 0.5) + p);

                // Hires
                if (!dhgr) {
                    phase = 2.0 * Math.PI * (subcarrier * (xx + 84 + 0.5) + p);
                }

                yuv_image[j] = raw_image[i] * 1.0 / 255.0;
                yuv_image[j+1] = raw_image[i+1] * 1.0 / 255.0 * Math.sin(phase);
                yuv_image[j+2] = raw_image[i+2] * 1.0 / 255.0 * Math.cos(phase);

                i += 4;
                j += 3;
            }
        }

        // Dump out the finalized image
        for (int yy = 0; yy<bi.getHeight(); yy++) {
            for (int xx = 0; xx<bi.getWidth(); xx++) {
                int index = (xx+yy*bi.getWidth())*3;

                double[] c = color_at(yuv_image,xx,yy);

                double y = c[0];
                double u = c[1];
                double v = c[2];

                int r = (int) ((y + 0.00000 * u + 1.13983 * v) * 255);
                int g = (int) ((y - 0.39465 * u - 0.58060 * v) * 255);
                int b = (int) ((y + 2.03211 * u + 0.00000 * v) * 255);

                if (r < 0) r = 0;
                if (r > 255) r = 255;

                if (g < 0) g = 0;
                if (g > 255) g = 255;

                if (b < 0) b = 0;
                if (b > 255) b = 255;

                int out_value = r*65536+g*256+b;
                bout.setRGB(xx,yy, out_value);
            }
        }        

        ImageIO.write(bout, "PNG", new File("test.png"));
    }

}
