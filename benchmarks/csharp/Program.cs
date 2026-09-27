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
        var xml = initial.ToString();
        using (var check = new StringReader(xml))
        {
            var decoded = serializer.Deserialize(check) as T ?? throw new Exception("Round trip failed");
            using var verify = new StringWriter();
            serializer.Serialize(verify, decoded);
            if (verify.ToString() != xml) throw new Exception("Round trip mismatch");
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
