package bench

import (
	"bytes"
	"encoding/xml"
	"fmt"
	"os"
	"testing"

	models "polyxml-bench-go/target/generated"
)

type baseline struct {
	XMLName xml.Name `xml:"Sensor"`
	ID      string   `xml:"Id"`
	Value   int32    `xml:"Value"`
}

func BenchmarkXML(b *testing.B) {
	for _, count := range []int{1, 1000} {
		b.Run(fmt.Sprintf("size=%d", count), func(b *testing.B) {
			generated := make([]models.Sensor, count)
			plain := make([]baseline, count)
			for i := range generated {
				generated[i] = models.Sensor{ID: fmt.Sprintf("sensor-%d", i), Value: int32(i)}
				plain[i] = baseline{ID: generated[i].ID, Value: generated[i].Value}
			}
			cases := []struct {
				name   string
				values any
				read   func([]byte, bool) error
			}{
				{"generated", generated, func(data []byte, verify bool) error {
					var v struct {
						Items []models.Sensor `xml:"Sensor"`
					}
					if err := xml.Unmarshal(data, &v); err != nil {
						return err
					}
					if verify {
						if len(v.Items) != count {
							return fmt.Errorf("generated count mismatch")
						}
						for i, sensor := range v.Items {
							if sensor.ID != generated[i].ID || sensor.Value != generated[i].Value {
								return fmt.Errorf("generated field mismatch at %d", i)
							}
						}
					}
					return nil
				}},
				{"baseline", plain, func(data []byte, verify bool) error {
					var v struct {
						Items []baseline `xml:"Sensor"`
					}
					if err := xml.Unmarshal(data, &v); err != nil {
						return err
					}
					if verify {
						if len(v.Items) != count {
							return fmt.Errorf("baseline count mismatch")
						}
						for i, sensor := range v.Items {
							if sensor.ID != plain[i].ID || sensor.Value != plain[i].Value {
								return fmt.Errorf("baseline field mismatch at %d", i)
							}
						}
					}
					return nil
				}},
			}
			if os.Getenv("BENCH_BASELINE_FIRST") == "1" {
				cases[0], cases[1] = cases[1], cases[0]
			}
			for _, tc := range cases {
				input, err := os.ReadFile(fmt.Sprintf("../workloads/sensor-batch/sensor-%d.xml", count))
				if err != nil {
					b.Fatal(err)
				}
				if err := tc.read(input, true); err != nil {
					b.Fatal(err)
				}
				output, err := xml.Marshal(struct {
					XMLName xml.Name `xml:"Batch"`
					Items   any      `xml:"Sensor"`
				}{Items: tc.values})
				if err != nil {
					b.Fatal(err)
				}
				if !bytes.Equal(output, input) {
					b.Fatal("writer output differs from shared fixture")
				}
				b.Run(tc.name+"/read", func(b *testing.B) {
					b.SetBytes(int64(len(input)))
					b.ReportAllocs()
					b.ResetTimer()
					for i := 0; i < b.N; i++ {
						if err := tc.read(input, false); err != nil {
							b.Fatal(err)
						}
					}
				})
				b.Run(tc.name+"/write", func(b *testing.B) {
					b.SetBytes(int64(len(input)))
					b.ReportAllocs()
					b.ResetTimer()
					for i := 0; i < b.N; i++ {
						if _, err := xml.Marshal(struct {
							XMLName xml.Name `xml:"Batch"`
							Items   any      `xml:"Sensor"`
						}{Items: tc.values}); err != nil {
							b.Fatal(err)
						}
					}
				})
			}
		})
	}
}
