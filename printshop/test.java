import java.io.*;
import java.util.*;
import javax.imageio.*;
import java.awt.image.*;
import com.itextpdf.text.Document;
import com.itextpdf.text.Image;
import com.itextpdf.text.Paragraph;
import com.itextpdf.text.pdf.PdfWriter;
import com.itextpdf.text.pdf.PdfContentByte;

public class test {
    public static void main(String args[]) throws Exception {
        new test().start(args);
    }

    public void start(String args[]) throws Exception {
        String filename = "test.bin";

        File file = new File(filename);
        DataInputStream is = new DataInputStream(new FileInputStream(file));

        byte[] buffer = new byte[(int)file.length()];

        is.readFully(buffer);

        int offset = 0;

        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        int row = 0;
        byte color = 0;
        int[] image_buffer = null;
        List<Integer> image_int = new ArrayList<Integer>();
        int columnSize = 0;
        
        while (offset < buffer.length) {
            if (buffer[offset] == 0xd) {
                offset++;
                continue;
            }

            if (buffer[offset] == 0xa) {
                offset++;
                if (image_buffer != null) {
                    row += 7;
                    //baos.write(image_buffer);
                    for (int value: image_buffer) {
                        image_int.add(value);
                    }
                    image_buffer = null;
                }
                continue;
            }            

            if (buffer[offset] == 0x1b) {
                switch (buffer[offset+1]) {
                case 0x41:
                    offset += 3;
                    continue;
                case 0x4c:
                    columnSize = buffer[offset+3]*256+buffer[2+offset];
                    byte[] image = getBuffer(buffer,offset,columnSize, color);
                    if (image_buffer == null) {
                        image_buffer = new int[image.length];
                        for (int i=0;i<image.length;i++) {
                            int out_color = 16777215;
                            if (image[i] != 127) {
                                out_color = getColor(image[i]);
                            }
                            image_buffer[i] = out_color;
                        }
                    } else {
                        for (int i=0;i<image.length;i++) {
                            int out_color = 16777215;
                            if (image[i] != 127) {
                                out_color = getColor(image[i]);
                            }
                            image_buffer[i] = (image_buffer[i] & out_color); 
                        }
                    }
                    offset += columnSize+4;
                    break;
                case 0x72:
                    color = buffer[offset+2];
                    offset += 3;
                    break;
                default: 
                    System.out.println("Cmd Offset="+offset+" Value="+buffer[offset+1]);
                    return;
                }
            } else {
                System.out.println("Offset="+offset+" Value="+buffer[offset]);
                return;
            }
        }

        BufferedImage bimage = new BufferedImage(columnSize,row,BufferedImage.TYPE_INT_RGB);
        
        for (int j=0;j<row;j++) {
            for (int i=0;i<columnSize;i++) {
                int value = (int) image_int.get(i+j*1024);
                bimage.setRGB(i,j,value);
            }
        }
        
        ImageIO.write((RenderedImage) bimage, "png", new File("test.png"));


        // Generate PDF file
        Document document = new Document();

        document.setMargins(10,10,10,10);

        PdfWriter writer = PdfWriter.getInstance(document, new FileOutputStream("test.pdf"));
        document.open();
        PdfContentByte pdfCB = new PdfContentByte(writer);
        Image image = Image.getInstance(pdfCB, bimage, 1);
        
        // A4 size is 595 x 842 (8.29 in by 11.69 in)
        // Letter size is 612 x 792 (8.5 in by 11 in)
        // By default the image is draw at 72 dpi, the printout is at 120 dpi
        //image.scaleToFit(595, 842);
        image.scalePercent(60,100);

        Paragraph p = new Paragraph();
        p.add(image);
        document.add(p);
        document.close();
    }

    public int getColor(byte color) {
        int out_color = 0;
        switch (color) {
        case 1:
            out_color = 255*256*256+255;
            break;
        case 2:
            out_color = 255*256+255;
            break;
        case 3:
            out_color = 238*256*256+130*256+238;
            break;
        case 4:
            out_color = 255*256*256+255*256;
            break;
        case 5:
            out_color = 255*256*256;
            break;
        case 6:
            out_color = 255*256;
            break;
        default:
            out_color = 0;
        }

        return out_color;
    }

    public byte[] getBuffer(byte[] buffer, int offset, int columnSize, byte color) {
        byte[] image = new byte[ columnSize*7 ];

        for (int i=0;i<image.length;i++) {
            image[i] = 127;
        }

        for (int i=0;i<columnSize;i++) {
        int mask = 0x40;
        int j=0;
        int value = buffer[offset+4+i];
        while (mask > 0) {
            if ((value & mask) > 0) {
                image[j*columnSize+i] = color;
            } else {
                image[j*columnSize+i] = 127;
            }

            j += 1;
            mask >>= 1;
        }
        }
        return image;
    }
}
