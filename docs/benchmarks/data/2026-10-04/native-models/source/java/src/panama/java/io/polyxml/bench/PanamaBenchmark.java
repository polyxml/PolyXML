package io.polyxml.bench;
import io.polyxml.PolyXML;
import java.io.ByteArrayInputStream;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.*;
import org.openjdk.jmh.infra.Blackhole;

@State(Scope.Thread)
@BenchmarkMode(Mode.Throughput)
@OutputTimeUnit(TimeUnit.SECONDS)
@Warmup(iterations=3,time=1)
@Measurement(iterations=5,time=1)
@Fork(value=2,jvmArgsAppend={"--enable-native-access=ALL-UNNAMED"})
public class PanamaBenchmark {
    @Param({"settlement","telemetry"}) public String workload;
    @Param({"10000"}) public int batchSize;
    private PolyXML.Schema schema;
    private PolyXML.Value value;
    private byte[][] documents;
    @Setup public void setup() throws Exception {
        var fixture=new BindingBenchmark(); fixture.workload=workload; fixture.batchSize=batchSize; fixture.setup(); documents=fixture.documents;
        var builder=new PolyXML.SchemaBuilder("Message");
        for(String name:new String[]{"id","status","timestamp","amount","latitude","longitude","payload"}) builder.addField(name,name,PolyXML.FieldKind.ELEMENT,PolyXML.ScalarType.STRING);
        for(int i=0;i<73;i++) builder.addField("optional"+i,"optional"+i,PolyXML.FieldKind.ELEMENT,PolyXML.ScalarType.STRING);
        schema=builder.build(); value=PolyXML.deserialize(documents[0],schema);
        byte[] xml=PolyXML.serialize("message",value,schema,0);
        if(!fixture.pojo.equals(io.polyxml.bench.pojo.MessageCodec.readXml(new ByteArrayInputStream(xml)))) throw new AssertionError("Panama round trip");
    }
    @TearDown public void close() { if(value!=null)value.close(); if(schema!=null)schema.close(); }
    @Benchmark public void panamaRead(Blackhole bh) { for(byte[] xml:documents) try(var parsed=PolyXML.deserialize(xml,schema)) { bh.consume(parsed.getField("id").getString().orElseThrow()); } }
    @Benchmark public void panamaWrite(Blackhole bh) { for(int i=0;i<batchSize;i++) bh.consume(PolyXML.serialize("message",value,schema,0)); }
}
