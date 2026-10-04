package io.polyxml.bench;
import org.junit.jupiter.api.Test;
public class BindingTest {
    @Test void allBackendsRoundTrip() throws Exception {
        for (String workload : new String[] {"settlement", "telemetry"}) {
            var state = new BindingBenchmark(); state.workload=workload; state.batchSize=2; state.setup();
        }
    }
}
