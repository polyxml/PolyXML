import generated.models.BatchType;
import generated.models.BatchTypeCodec;
import java.io.ByteArrayInputStream;
import java.nio.file.Files;
import java.nio.file.Path;

public final class SharedSensorBench {
    private static volatile long sink;

    private static BatchType decode(byte[] xml) throws Exception {
        return BatchTypeCodec.readXml(new ByteArrayInputStream(xml));
    }

    private static void measure(int count, int iterations) throws Exception {
        byte[] xml = Files.readAllBytes(Path.of("sensor-" + count + ".xml"));
        BatchType first = decode(xml);
        int mid = count / 2;
        if (first.getSensor().size() != count ||
            !first.getSensor().get(0).getId().equals("sensor-0") ||
            first.getSensor().get(0).getValue() != 0 ||
            !first.getSensor().get(mid).getId().equals("sensor-" + mid) ||
            first.getSensor().get(mid).getValue() != mid ||
            !first.getSensor().get(count - 1).getId().equals("sensor-" + (count - 1)) ||
            first.getSensor().get(count - 1).getValue() != count - 1) {
            throw new IllegalStateException("Generated Java decoder returned unexpected values");
        }
        for (int i = 0; i < (count == 1 ? 100000 : 1000); i++) {
            sink += decode(xml).getSensor().size();
        }
        for (int repeat = 0; repeat < 5; repeat++) {
            long start = System.nanoTime();
            for (int i = 0; i < iterations; i++) {
                sink += decode(xml).getSensor().size();
            }
            double ns = (double) (System.nanoTime() - start) / iterations;
            System.out.printf("java,size=%d,repeat=%d,ns/op=%.1f,xml_bytes=%d,sink=%d%n",
                    count, repeat, ns, xml.length, sink);
        }
    }

    public static void main(String[] args) throws Exception {
        System.out.println("java_version=" + System.getProperty("java.version"));
        measure(1, 100000);
        measure(1000, 1000);
    }
}
