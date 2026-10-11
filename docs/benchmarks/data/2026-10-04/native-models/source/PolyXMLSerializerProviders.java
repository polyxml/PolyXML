public class PolyXMLSerializerProviders {
    public static void main(String[] args) {
        System.out.println("input=" + javax.xml.stream.XMLInputFactory.newFactory().getClass().getName());
        System.out.println("output=" + javax.xml.stream.XMLOutputFactory.newFactory().getClass().getName());
    }
}
