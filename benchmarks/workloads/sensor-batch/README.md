# Shared sensor batch workload

`sensor-1.xml` and `sensor-1000.xml` are byte-identical input fixtures for the
Go, C++, and C# benchmark readers. Each `<Sensor>` has an `Id` of `sensor-N`
and a numeric `Value` of `N`, from zero to count minus one. Both files have a
`<Batch>` root and no XML declaration, namespace, or whitespace between nodes.

Readers must materialize the same number of sensor models and verify the final
ID and value before timing. Writers start from equivalent preconstructed models
and may emit runtime-specific namespace declarations; compare logical values
and record the output byte length separately. This workload measures different
model/serializer combinations and does not make the implementations of the
languages directly comparable in speed.
