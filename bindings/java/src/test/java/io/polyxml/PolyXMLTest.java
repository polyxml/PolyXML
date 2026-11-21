package io.polyxml;

import org.junit.jupiter.api.Test;
import java.util.Map;
import static org.junit.jupiter.api.Assertions.*;

public class PolyXMLTest {

    @Test
    public void testVersion() {
        String ver = PolyXML.version();
        assertNotNull(ver);
        assertFalse(ver.isEmpty());
    }

    @Test
    public void testEnumDefinitions() {
        assertEquals(0, PolyXML.FieldKind.ATTRIBUTE.ordinal());
        assertEquals(1, PolyXML.FieldKind.ELEMENT.ordinal());
        assertEquals(2, PolyXML.FieldKind.TEXT.ordinal());

        assertEquals(0, PolyXML.ScalarType.STRING.ordinal());
        assertEquals(1, PolyXML.ScalarType.INT.ordinal());
        assertEquals(2, PolyXML.ScalarType.FLOAT.ordinal());
        assertEquals(3, PolyXML.ScalarType.BOOL.ordinal());
    }

    @Test
    public void testSchemaBuilderAndRoundtrip() {
        try (PolyXML.Schema schema = new PolyXML.SchemaBuilder("Sensor")
                .addField("id", "id", PolyXML.FieldKind.ATTRIBUTE, PolyXML.ScalarType.INT)
                .addField("name", "name", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING)
                .addField("reading", "reading", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.FLOAT)
                .addField("calibrated", "calibrated", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.BOOL)
                .build()) {

            assertNotNull(schema.handle());

            String xml = "<Sensor id=\"101\"><name>Barometer</name><reading>1013.25</reading><calibrated>true</calibrated></Sensor>";
            try (PolyXML.Value val = PolyXML.deserialize(xml, schema)) {
                assertNotNull(val);
                assertFalse(val.isNull());

                PolyXML.Value idVal = val.getField("id");
                assertNotNull(idVal);
                assertEquals(101L, idVal.getInt().orElse(-1L));

                PolyXML.Value nameVal = val.getField("name");
                assertNotNull(nameVal);
                assertEquals("Barometer", nameVal.getString().orElse(""));

                PolyXML.Value readVal = val.getField("reading");
                assertNotNull(readVal);
                assertEquals(1013.25, readVal.getFloat().orElse(0.0), 0.001);

                PolyXML.Value calVal = val.getField("calibrated");
                assertNotNull(calVal);
                assertTrue(calVal.getBool().orElse(false));

                String serialized = PolyXML.serializeToString("Sensor", val, schema, 2);
                assertTrue(serialized.contains("id=\"101\""));
                assertTrue(serialized.contains("<name>Barometer</name>"));
                assertTrue(serialized.contains("<reading>1013.25</reading>"));
                assertTrue(serialized.contains("<calibrated>true</calibrated>"));
            }
        }
    }

    @Test
    public void testNamespacedSchemaAndSerialization() {
        try (PolyXML.Schema schema = new PolyXML.SchemaBuilder("Order")
                .setNamespace("https://example.com/orders")
                .addField("id", "id", PolyXML.FieldKind.ATTRIBUTE, PolyXML.ScalarType.INT)
                .addField("item", "item", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING, "https://example.com/items")
                .build()) {

            String xml = "<ns0:Order xmlns:ns0=\"https://example.com/orders\" xmlns:ns1=\"https://example.com/items\" id=\"888\"><ns1:item>JavaGadget</ns1:item></ns0:Order>";
            try (PolyXML.Value val = PolyXML.deserialize(xml, schema)) {
                assertNotNull(val);
                assertEquals(888L, val.getField("id").getInt().orElse(-1L));
                assertEquals("JavaGadget", val.getField("item").getString().orElse(""));

                Map<String, String> nsMap = Map.of(
                    "ord", "https://example.com/orders",
                    "itm", "https://example.com/items"
                );

                byte[] bytes = PolyXML.serializeWithOptions("Order", val, schema, 0, true, nsMap);
                String outXml = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);

                assertTrue(outXml.contains("xmlns:ord=\"https://example.com/orders\""));
                assertTrue(outXml.contains("xmlns:itm=\"https://example.com/items\""));
                assertTrue(outXml.contains("<ord:Order"));
                assertTrue(outXml.contains("<itm:item>JavaGadget</itm:item>"));
            }
        }
    }

    @Test
    public void testConformanceAtomFeed() throws Exception {
        java.nio.file.Path[] candidates = new java.nio.file.Path[] {
            java.nio.file.Paths.get("../../tests/fixtures/samples/atom_feed.xml"),
            java.nio.file.Paths.get("tests/fixtures/samples/atom_feed.xml")
        };
        java.nio.file.Path target = null;
        for (java.nio.file.Path p : candidates) {
            if (java.nio.file.Files.exists(p)) {
                target = p;
                break;
            }
        }
        if (target == null) {
            return;
        }

        String xml = java.nio.file.Files.readString(target);
        try (PolyXML.Schema schema = new PolyXML.SchemaBuilder("feed")
                .setNamespace("http://www.w3.org/2005/Atom")
                .addField("title", "title", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING)
                .addField("id", "id", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING)
                .build()) {

            try (PolyXML.Value val = PolyXML.deserialize(xml, schema)) {
                assertNotNull(val);
                assertEquals("PolyXML Engineering Updates", val.getField("title").getString().orElse(""));

                Map<String, String> nsMap = Map.of("", "http://www.w3.org/2005/Atom");
                byte[] bytes = PolyXML.serializeWithOptions("feed", val, schema, 2, true, nsMap);
                String out = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
                assertTrue(out.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
                assertTrue(out.contains("<title>PolyXML Engineering Updates</title>"));
            }
        }
    }
}
