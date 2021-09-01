package main

import (
	"os"
	"testing"
)

func TestBlock(t *testing.T) {
	for _, p := range []string{"test-data/output-1.json", "test-data/output-2.json"} {
		t.Run(p, func(t *testing.T) {
			f, err := os.Open(p)
			if err != nil {
				t.Fatal(err)
			}
			defer f.Close()
			_, err = parse(f)
			if err != nil {
				t.Fatal(err)
			}
		})
	}
}
