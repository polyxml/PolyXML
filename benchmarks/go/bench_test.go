package bench

import (
	"encoding/xml"
	"fmt"
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
			for _, tc := range []struct {
				name   string
				values any
				read   func([]byte) error
			}{
				{"generated", generated, func(data []byte) error {
					var v struct {
						Items []models.Sensor `xml:"Sensor"`
					}
					if err := xml.Unmarshal(data, &v); err != nil {
						return err
					}
					if len(v.Items) != count || v.Items[count-1].ID != generated[count-1].ID {
						return fmt.Errorf("generated round trip mismatch")
					}
					return nil
				}},
				{"baseline", plain, func(data []byte) error {
					var v struct {
						Items []baseline `xml:"Sensor"`
					}
					if err := xml.Unmarshal(data, &v); err != nil {
						return err
					}
					if len(v.Items) != count || v.Items[count-1].ID != plain[count-1].ID {
						return fmt.Errorf("baseline round trip mismatch")
					}
					return nil
				}},
			} {
				input, err := xml.Marshal(struct {
					XMLName xml.Name `xml:"Batch"`
					Items   any      `xml:"Sensor"`
				}{Items: tc.values})
				if err != nil {
					b.Fatal(err)
				}
				if err := tc.read(input); err != nil {
					b.Fatal(err)
				}
				b.Run(tc.name+"/read", func(b *testing.B) {
					b.SetBytes(int64(len(input)))
					b.ReportAllocs()
					b.ResetTimer()
					for i := 0; i < b.N; i++ {
						if err := tc.read(input); err != nil {
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
