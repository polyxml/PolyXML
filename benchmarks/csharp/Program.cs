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
            int mid = count / 2;
            if (decoded is Batch<Sensor> generated && (
                generated.Sensors.Length != count ||
                generated.Sensors[0].Id != "sensor-0" || generated.Sensors[0].Value != 0 ||
                generated.Sensors[mid].Id != $"sensor-{mid}" || generated.Sensors[mid].Value != mid ||
                generated.Sensors[^1].Id != $"sensor-{count-1}" || generated.Sensors[^1].Value != count-1))
                throw new Exception("Generated batch mismatch");
            if (decoded is Batch<BaselineSensor> baseline && (
                baseline.Sensors.Length != count ||
                baseline.Sensors[0].Id != "sensor-0" || baseline.Sensors[0].Value != 0 ||
                baseline.Sensors[mid].Id != $"sensor-{mid}" || baseline.Sensors[mid].Value != mid ||
                baseline.Sensors[^1].Id != $"sensor-{count-1}" || baseline.Sensors[^1].Value != count-1))
                throw new Exception("Baseline batch mismatch");
            using var verify = new StringWriter();
            serializer.Serialize(verify, decoded);
            if (verify.ToString() != initial.ToString()) throw new Exception("Round trip mismatch");
        }
        for (var i = 0; i < 100; i++) { using var reader = new StringReader(xml); _ = serializer.Deserialize(reader); using var writer = new StringWriter(); serializer.Serialize(writer, value); }
        foreach (var operation in new[] { "read", "write" })
        {
            GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
            var bytes = GC.GetAllocatedBytesForCurrentThread();
            var timer = Stopwatch.StartNew();
            for (var i = 0; i < Iterations; i++)
            {
                if (operation == "read") { using var reader = new StringReader(xml); _ = serializer.Deserialize(reader); }
                else { using var writer = new StringWriter(); serializer.Serialize(writer, value); }
            }
            timer.Stop();
            Console.WriteLine($"{name},size={count},{operation},ns/op={timer.Elapsed.TotalNanoseconds / Iterations:F0},B/op={(GC.GetAllocatedBytesForCurrentThread()-bytes)/Iterations},xml_bytes={System.Text.Encoding.UTF8.GetByteCount(xml)}");
        }
    }

    public static void Main()
    {
        Console.WriteLine($"dotnet={Environment.Version},os={System.Runtime.InteropServices.RuntimeInformation.OSDescription},cpu={Environment.ProcessorCount},iterations={Iterations}");
        foreach (var count in new[] { 1, 1000 })
        {
            Measure("generated", count, n => new Batch<Sensor> { Sensors = Enumerable.Range(0, n).Select(i => new Sensor { Id = $"sensor-{i}", Value = i }).ToArray() });
            Measure("baseline", count, n => new Batch<BaselineSensor> { Sensors = Enumerable.Range(0, n).Select(i => new BaselineSensor { Id = $"sensor-{i}", Value = i }).ToArray() });
        }
    }
}
