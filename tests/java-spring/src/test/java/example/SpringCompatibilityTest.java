package example;

import static org.junit.jupiter.api.Assertions.*;

import jakarta.validation.Valid;
import java.math.BigDecimal;
import java.net.URI;
import java.net.http.*;
import org.junit.jupiter.api.Test;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.boot.web.server.context.WebServerApplicationContext;
import org.springframework.web.bind.annotation.*;
import tools.jackson.databind.json.JsonMapper;
import tools.jackson.dataformat.xml.XmlMapper;

class SpringCompatibilityTest {
  static final String XML =
      "<Order xmlns=\"urn:orders\""
          + " id=\"A1\"><customer>Ada</customer><status>NEW</status><created>2026-10-02T12:00:00Z</created><amount"
          + " currency=\"USD\">12.50</amount><item>one</item><item>two</item></Order>";

  @SpringBootApplication
  @RestController
  static class App {
    @org.springframework.context.annotation.Bean
    org.springframework.boot.jackson.autoconfigure.XmlMapperBuilderCustomizer xmlTextName() {
      return builder -> builder.nameForTextElement("value");
    }

    @PostMapping("/pojo")
    example.pojo.Order pojo(@Valid @RequestBody example.pojo.Order order) {
      return order;
    }

    @PostMapping("/record")
    example.record.Order record(@Valid @RequestBody example.record.Order order) {
      return order;
    }
  }

  static void validateXml(String content) throws Exception {
    var factory =
        javax.xml.validation.SchemaFactory.newInstance(
            javax.xml.XMLConstants.W3C_XML_SCHEMA_NS_URI);
    var schema = factory.newSchema(new java.io.File("schema.xsd"));
    schema
        .newValidator()
        .validate(new javax.xml.transform.stream.StreamSource(new java.io.StringReader(content)));
  }

  @Test
  void mapperAndDirectCodecRoundTrips() throws Exception {
    var xml = XmlMapper.builder().nameForTextElement("value").build();
    var json = JsonMapper.builder().build();
    for (var type : new Class<?>[] {example.pojo.Order.class, example.record.Order.class}) {
      var value = xml.readValue(XML, type);
      var encoded = xml.writeValueAsString(value);
      assertTrue(encoded.contains("currency=\"USD\""), encoded);
      assertFalse(encoded.contains("<value>"), encoded);
      validateXml(encoded);
      assertEquals(value, xml.readValue(encoded, type));
      assertEquals(value, json.readValue(json.writeValueAsString(value), type));
    }
    var pojo = xml.readValue(XML, example.pojo.Order.class);
    assertEquals("Ada", pojo.getCustomer().getValue());
    assertEquals(2, pojo.getItem().size());
    assertEquals(new BigDecimal("12.50"), pojo.getAmount().getValue());
    assertNull(pojo.getNote());
    var record = xml.readValue(XML, example.record.Order.class);
    assertTrue(record.note().isEmpty());
    assertEquals(
        pojo,
        example.pojo.OrderCodec.readXml(
            new java.io.ByteArrayInputStream(
                XML.getBytes(java.nio.charset.StandardCharsets.UTF_8))));
    assertEquals(
        record,
        example.record.OrderCodec.readXml(
            new java.io.ByteArrayInputStream(
                XML.getBytes(java.nio.charset.StandardCharsets.UTF_8))));
    var output = new java.io.ByteArrayOutputStream();
    example.pojo.OrderCodec.writeXml(pojo, output);
    assertEquals(
        pojo,
        example.pojo.OrderCodec.readXml(new java.io.ByteArrayInputStream(output.toByteArray())));
    output.reset();
    example.record.OrderCodec.writeXml(record, output);
    assertEquals(
        record,
        example.record.OrderCodec.readXml(new java.io.ByteArrayInputStream(output.toByteArray())));
    assertEquals("B2", example.record.Order.builder().id("B2").build().id());
    assertEquals("B2", example.pojo.Order.builder().id("B2").build().getId());
  }

  @Test
  void springHttpConvertersAndValidation() throws Exception {
    try (var context =
        SpringApplication.run(App.class, "--server.port=0", "--spring.main.banner-mode=off")) {
      int port = ((WebServerApplicationContext) context).getWebServer().getPort();
      var client = HttpClient.newHttpClient();
      var xml = XmlMapper.builder().nameForTextElement("value").build();
      var json = context.getBean(JsonMapper.class);
      for (var style : new String[] {"pojo", "record"}) {
        Class<?> type =
            style.equals("pojo") ? example.pojo.Order.class : example.record.Order.class;
        var value = xml.readValue(XML, type);
        for (var media : new String[] {"application/xml", "application/json"}) {
          var mapper = media.endsWith("xml") ? xml : json;
          var body = media.endsWith("xml") ? XML : mapper.writeValueAsString(value);
          var request =
              HttpRequest.newBuilder(URI.create("http://localhost:" + port + "/" + style))
                  .header("Content-Type", media)
                  .header("Accept", media)
                  .POST(HttpRequest.BodyPublishers.ofString(body))
                  .build();
          var response = client.send(request, HttpResponse.BodyHandlers.ofString());
          assertEquals(200, response.statusCode(), response.body());
          assertEquals(value, mapper.readValue(response.body(), type));
          if (media.endsWith("xml")) validateXml(response.body());
          var invalid =
              HttpRequest.newBuilder(request.uri())
                  .header("Content-Type", "application/json")
                  .POST(HttpRequest.BodyPublishers.ofString("{}"))
                  .build();
          assertEquals(
              400, client.send(invalid, HttpResponse.BodyHandlers.ofString()).statusCode());
        }
      }
    }
  }
}
