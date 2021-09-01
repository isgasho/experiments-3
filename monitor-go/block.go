package main

import (
	"encoding/json"
	"fmt"
	"io"
	"strconv"
	"strings"
)

func parse(rd io.Reader) (b Block, err error) {
	err = json.NewDecoder(rd).Decode(&b)
	return b, err
}

type Block struct {
	ChainID string `json:"chain_id"`
	Type    string `json:"type"`
	Data    Data   `json:"data"`
}

type Data struct {
	Txs    []Tx     `json:"txs"`
	Supply []Supply `json:"supply"`
}

type Tx struct {
	Height string `json:"height"`
	Logs   []Log  `json:"logs"`
}

type Log struct {
	Events []Event `json:"events"`
}

type Event struct {
	Type       string      `json:"type"`
	Attributes []Attribute `json:"attributes"`
}

type Attribute struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

type Supply struct {
	Denom  string `json:"denom"`
	Amount string `json:"amount"`
}

type Prices struct {
	Prices []Price `json:"prices"`
}

type Price struct {
	Denom  string  `json:"denom"`
	Price  float64 `json:"price"`
	Volume uint64  `json:"volume"`
}

func (b Block) getExchangeRates() (rates map[string]float64, err error) {
	exRates := ""
done:
	for _, tx := range b.Data.Txs {
		for _, lv := range tx.Logs {
			for _, ev := range lv.Events {
				if ev.Type != "aggregate_vote" {
					continue
				}
				for _, attr := range ev.Attributes {
					if attr.Key != "exchange_rates" {
						continue
					}
					exRates = attr.Value
					break done
				}
			}
		}
	}
	if exRates == "" {
		return nil, nil
	}
	rates = make(map[string]float64)
	for _, rate := range strings.Split(exRates, ",") {
		ss := strings.SplitN(rate, "u", 2)
		if len(ss) != 2 {
			return nil, fmt.Errorf("invalid exchange_rates %q", rate)
		}
		var f float64
		f, err = strconv.ParseFloat(ss[0], 64)
		if err != nil {
			return nil, err
		}
		rates["u"+ss[1]] = f
	}
	rates["uluna"] = 1.0
	return rates, nil
}

func (b Block) getSupply() (supplies map[string]uint64, err error) {
	supplies = make(map[string]uint64)
	for _, sp := range b.Data.Supply {
		av, err := strconv.ParseUint(sp.Amount, 10, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid supply amount %q for %q", sp.Amount, sp.Denom)
		}
		supplies[sp.Denom] = av
	}
	return supplies, nil
}
