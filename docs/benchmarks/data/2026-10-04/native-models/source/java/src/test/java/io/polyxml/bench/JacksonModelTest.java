package io.polyxml.bench;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.xml.XmlMapper;
import io.polyxml.bench.cases.*;
import java.io.*;
public class JacksonModelTest {
    @Test void beansWithInheritanceAndRenamedFields() throws Exception {
        Item item=Item.builder().id(42).displayName("a<&").active(true).code(new Code("OK")).build();
        item.getTags().add("one"); item.getTags().add("two");
        item.setChild(Item.builder().id(7).displayName("child").build());
        XmlMapper xml=new XmlMapper();
        byte[] encoded=xml.writeValueAsBytes(item);
        assertEquals(item,xml.readValue(encoded,Item.class));
        assertEquals(item,ItemCodec.readXml(new ByteArrayInputStream(encoded)));
        var direct=new ByteArrayOutputStream(); ItemCodec.writeXml(item,direct);
        assertEquals(item,xml.readValue(direct.toByteArray(),Item.class));
        ObjectMapper json=new ObjectMapper();
        assertEquals(item,json.readValue(json.writeValueAsBytes(item),Item.class));
        assertFalse(new String(encoded,java.nio.charset.StandardCharsets.UTF_8).contains("displayName"));
    }
}
