/** Record the StAX provider actually selected by the shaded benchmark classpath. */
public class Providers {
    public static void main(String[] args) {
        System.out.println("input=" + javax.xml.stream.XMLInputFactory.newFactory().getClass().getName());
        System.out.println("output=" + javax.xml.stream.XMLOutputFactory.newFactory().getClass().getName());
    }
}
