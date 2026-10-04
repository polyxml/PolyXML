using System.Diagnostics;
using System.Xml.Serialization;
using Generated;

[XmlRoot("Sensor")]
public sealed class BaselineSensor
{
    [XmlElement("Id")] public string Id { get; set; } = "";
    [XmlElement("Value")] public int Value { get; set; }
}

[XmlRoot("Batch")]
public sealed class Batch<T>
{
    [XmlElement("Sensor")] public T[] Sensors { get; set; } = Array.Empty<T>();
}

public static class Program
{
    static readonly int Iterations = int.TryParse(Environment.GetEnvironmentVariable("BENCH_ITERATIONS"), out var n) ? n : 1000;

    static void Measure<T>(string name, int count, Func<int,T> make) where T : class
    {
        var serializer = new XmlSerializer(typeof(T));
        var value = make(count);
        using var initial = new StringWriter();
        serializer.Serialize(initial, value);
        var xml = File.ReadAllText($"../workloads/sensor-batch/sensor-{count}.xml");
        using (var check = new StringReader(xml))
        {
            var decoded = serializer.Deserialize(check) as T ?? throw new Exception("Round trip failed");
            var fields = decoded switch {
                Batch<Sensor> generated => generated.Sensors.Select(s => (s.Id, s.Value)),
                Batch<BaselineSensor> baseline => baseline.Sensors.Select(s => (s.Id, s.Value)),
                _ => throw new Exception("Unknown model")
            };
            int index = 0;
            foreach (var (id, number) in fields) {
                if (id != $"sensor-{index}" || number != index) throw new Exception($"Field mismatch at {index}");
                index++;
            }
            if (index != count) throw new Exception("Sensor count mismatch");
            using var verify = new StringWriter();
            serializer.Serialize(verify, decoded);
            if (verify.ToString() != initial.ToString()) throw new Exception("Round trip mismatch");
        }
        var warmup = Stopwatch.StartNew();
        int warmed = 0;
        int warmupMs = int.TryParse(Environment.GetEnvironmentVariable("BENCH_WARMUP_MS"), out var ms) ? ms : 2000;
        while (warmed < 500 || warmup.ElapsedMilliseconds < warmupMs) {
            using var reader = new StringReader(xml);
            GC.KeepAlive(serializer.Deserialize(reader));
            using var writer = new StringWriter();
            serializer.Serialize(writer, value);
            GC.KeepAlive(writer.ToString());
            warmed++;
        }
        warmup.Stop();
        foreach (var operation in new[] { "read", "write" })
        {
            GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
            var bytes = GC.GetAllocatedBytesForCurrentThread();
            var timer = Stopwatch.StartNew();
            for (var i = 0; i < Iterations; i++)
            {
                if (operation == "read") { using var reader = new StringReader(xml); GC.KeepAlive(serializer.Deserialize(reader)); }
                else { using var writer = new StringWriter(); serializer.Serialize(writer, value); GC.KeepAlive(writer.ToString()); }
            }
            timer.Stop();
            Console.WriteLine($"{name},size={count},{operation},ns/op={timer.Elapsed.TotalNanoseconds / Iterations:F0},B/op={(GC.GetAllocatedBytesForCurrentThread()-bytes)/Iterations},xml_bytes={System.Text.Encoding.UTF8.GetByteCount(xml)},output_chars={initial.ToString().Length},warmup_iterations={warmed},warmup_ms={warmup.ElapsedMilliseconds}");
        }
    }

    public static void Main()
    {
        Console.WriteLine($"dotnet={Environment.Version},os={System.Runtime.InteropServices.RuntimeInformation.OSDescription},cpu={Environment.ProcessorCount},iterations={Iterations}");
        foreach (var count in new[] { 1, 1000 })
        {
            Action generated = () => Measure("generated", count, n => new Batch<Sensor> { Sensors = Enumerable.Range(0, n).Select(i => new Sensor { Id = $"sensor-{i}", Value = i }).ToArray() });
            Action baseline = () => Measure("baseline", count, n => new Batch<BaselineSensor> { Sensors = Enumerable.Range(0, n).Select(i => new BaselineSensor { Id = $"sensor-{i}", Value = i }).ToArray() });
            if (Environment.GetEnvironmentVariable("BENCH_BASELINE_FIRST") == "1") { baseline(); generated(); }
            else { generated(); baseline(); }
        }
    }
}
