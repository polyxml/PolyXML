package io.polyxml.bench;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.xml.XmlMapper;
import com.fasterxml.jackson.datatype.jdk8.Jdk8Module;
import jakarta.xml.bind.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.*;
import org.openjdk.jmh.infra.Blackhole;
import io.polyxml.bench.pojo.Message;
import io.polyxml.bench.pojo.MessageCodec;

/** Each operation processes a whole batch; divide time/allocation by batchSize for per-message values. */
@State(Scope.Thread)
@BenchmarkMode(Mode.Throughput)
@OutputTimeUnit(TimeUnit.SECONDS)
@Warmup(iterations = 3, time = 1)
@Measurement(iterations = 5, time = 1)
@Fork(2)
public class BindingBenchmark {
    @Param({"settlement", "telemetry"}) public String workload;
    @Param({"10000"}) public int batchSize;
    public byte[][] documents;
    public Message pojo;
    public io.polyxml.bench.records.Message record;
    public io.polyxml.bench.jaxb.Message legacy;
    public XmlMapper mapper;
    public Marshaller marshaller;
    public Unmarshaller unmarshaller;
    /** Shared StAX factories for the *Reuse benchmarks; per-thread in the generated codecs. */
    public javax.xml.stream.XMLInputFactory sharedInput;
    public javax.xml.stream.XMLOutputFactory sharedOutput;
    private final javax.xml.namespace.QName root = new javax.xml.namespace.QName("message");

    @Setup(Level.Trial)
    public void setup() throws Exception {
        mapper = XmlMapper.builder().addModule(new Jdk8Module()).build();
        sharedInput = javax.xml.stream.XMLInputFactory.newFactory();
        sharedInput.setProperty(javax.xml.stream.XMLInputFactory.SUPPORT_DTD, false);
        sharedInput.setProperty("javax.xml.stream.isSupportingExternalEntities", false);
        sharedOutput = javax.xml.stream.XMLOutputFactory.newFactory();
        JAXBContext context = JAXBContext.newInstance(io.polyxml.bench.jaxb.Message.class);
        marshaller = context.createMarshaller();
        unmarshaller = context.createUnmarshaller();
        documents = new byte[batchSize][];
        for (int i = 0; i < batchSize; i++) {
            // Scalar projections, not official ISO 20022/UCI schemas. Approximately 2.5 KB / 0.7 KB per message.
            String payload = "x".repeat(workload.equals("settlement") ? 2200 : 400);
            String xml = "<message><id>" + i + "</id><status>NEW</status><timestamp>2026-01-01T00:00:00Z</timestamp>"
                + "<amount>123.45</amount><latitude>40.0</latitude><longitude>-87.0</longitude><payload>" + payload + "</payload></message>";
            documents[i] = xml.getBytes(StandardCharsets.UTF_8);
        }
        pojo = MessageCodec.readXml(new ByteArrayInputStream(documents[0]));
        record = io.polyxml.bench.records.MessageCodec.readXml(new ByteArrayInputStream(documents[0]));
        legacy = unmarshaller.unmarshal(new javax.xml.transform.stream.StreamSource(new ByteArrayInputStream(documents[0])), io.polyxml.bench.jaxb.Message.class).getValue();
        verify();
    }
    public void verify() throws Exception {
        // Check every input and field across the matched typed readers before timing.
        for (byte[] xml : documents) {
            var expected = MessageCodec.readXml(new ByteArrayInputStream(xml));
            if (!expected.equals(mapper.readValue(xml, Message.class))) throw new AssertionError("Jackson input mismatch");
            var decoded = unmarshaller.unmarshal(new javax.xml.transform.stream.StreamSource(new ByteArrayInputStream(xml)), io.polyxml.bench.jaxb.Message.class).getValue();
            var encoded = new ByteArrayOutputStream();
            marshaller.marshal(new JAXBElement<>(root, io.polyxml.bench.jaxb.Message.class, decoded), encoded);
            if (!expected.equals(MessageCodec.readXml(new ByteArrayInputStream(encoded.toByteArray())))) throw new AssertionError("JAXB input mismatch");
        }
        var direct = new ByteArrayOutputStream(); MessageCodec.writeXml(pojo, direct);
        if (!pojo.equals(MessageCodec.readXml(new ByteArrayInputStream(direct.toByteArray())))) throw new AssertionError("direct round trip");
        if (!pojo.equals(mapper.readValue(mapper.writeValueAsBytes(pojo), Message.class))) throw new AssertionError("Jackson round trip");
        ObjectMapper json = new ObjectMapper().registerModule(new Jdk8Module());
        if (!pojo.equals(json.readValue(json.writeValueAsBytes(pojo), Message.class))) throw new AssertionError("JSON round trip");
        var jaxb = new ByteArrayOutputStream(); marshaller.marshal(new JAXBElement<>(root, io.polyxml.bench.jaxb.Message.class, legacy), jaxb);
        if (!pojo.equals(MessageCodec.readXml(new ByteArrayInputStream(jaxb.toByteArray())))) throw new AssertionError("JAXB round trip");
        long inputBytes = 0; for (byte[] xml : documents) inputBytes += xml.length;
        System.out.printf("fixture: workload=%s,batch=%d,input_bytes=%d,direct_output_bytes=%d,jackson_output_bytes=%d,jaxb_output_bytes=%d%n",
            workload, batchSize, inputBytes, direct.size(), mapper.writeValueAsBytes(pojo).length, jaxb.size());
        var records = new ByteArrayOutputStream(); io.polyxml.bench.records.MessageCodec.writeXml(record,records);
        if (!pojo.equals(MessageCodec.readXml(new ByteArrayInputStream(records.toByteArray())))) throw new AssertionError("record round trip");
    }
    @Benchmark public void directRead(Blackhole bh) throws Exception { for (byte[] xml : documents) bh.consume(MessageCodec.readXml(new ByteArrayInputStream(xml))); }
    @Benchmark public void directReadReuse(Blackhole bh) throws Exception { for (byte[] xml : documents) { var reader=sharedInput.createXMLStreamReader(new ByteArrayInputStream(xml)); try { bh.consume(MessageCodec.readXml(reader)); } finally { reader.close(); } } }
    @Benchmark public void recordRead(Blackhole bh) throws Exception { for (byte[] xml : documents) bh.consume(io.polyxml.bench.records.MessageCodec.readXml(new ByteArrayInputStream(xml))); }
    @Benchmark public void jacksonRead(Blackhole bh) throws Exception { for (byte[] xml : documents) bh.consume(mapper.readValue(xml,Message.class)); }
    @Benchmark public void jaxbRead(Blackhole bh) throws Exception { for (byte[] xml : documents) bh.consume(unmarshaller.unmarshal(new javax.xml.transform.stream.StreamSource(new ByteArrayInputStream(xml)),io.polyxml.bench.jaxb.Message.class).getValue()); }
    @Benchmark public void directWrite(Blackhole bh) throws Exception { for (int i=0;i<batchSize;i++) { var out=new ByteArrayOutputStream(); MessageCodec.writeXml(pojo,out); bh.consume(out.toByteArray()); } }
    @Benchmark public void directWriteReuse(Blackhole bh) throws Exception { for (int i=0;i<batchSize;i++) { var out=new ByteArrayOutputStream(); var writer=sharedOutput.createXMLStreamWriter(out,"UTF-8"); try { MessageCodec.writeXml(pojo,writer); writer.flush(); } finally { writer.close(); } bh.consume(out.toByteArray()); } }
    @Benchmark public void recordWrite(Blackhole bh) throws Exception { for (int i=0;i<batchSize;i++) { var out=new ByteArrayOutputStream(); io.polyxml.bench.records.MessageCodec.writeXml(record,out); bh.consume(out.toByteArray()); } }
    @Benchmark public void jacksonWrite(Blackhole bh) throws Exception { for (int i=0;i<batchSize;i++) bh.consume(mapper.writeValueAsBytes(pojo)); }
    @Benchmark public void jaxbWrite(Blackhole bh) throws Exception { for (int i=0;i<batchSize;i++) { var out=new ByteArrayOutputStream(); marshaller.marshal(new JAXBElement<>(root,io.polyxml.bench.jaxb.Message.class,legacy),out); bh.consume(out.toByteArray()); } }
    @Benchmark public void pojoMutation(Blackhole bh) throws Exception { for(byte[] xml:documents) { var value=MessageCodec.readXml(new ByteArrayInputStream(xml)); value.setStatus("PROCESSED"); var out=new ByteArrayOutputStream(); MessageCodec.writeXml(value,out); bh.consume(out.toByteArray()); } }
    @Benchmark public void recordMutation(Blackhole bh) throws Exception { for(byte[] xml:documents) { var value=io.polyxml.bench.records.MessageCodec.readXml(new ByteArrayInputStream(xml)); var updated=RecordMutation.withStatus(value,"PROCESSED"); var out=new ByteArrayOutputStream(); io.polyxml.bench.records.MessageCodec.writeXml(updated,out); bh.consume(out.toByteArray()); } }
}
