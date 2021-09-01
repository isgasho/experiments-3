package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

// Feeder feeds the block data on the "fetch" request.
type Feeder interface {
	prices() (ps Prices, err error)
	stop()
}

type feeder struct {
	requestTimeout time.Duration

	readyc chan struct{}
	stopc  chan struct{}

	mu   sync.RWMutex
	conn *websocket.Conn

	// caches the last known state
	// e.g., not every block has "aggregate_vote"
	lastExchangeRates map[string]float64
	lastSupply        map[string]uint64
}

func newFeeder(timeout time.Duration, observerURL string) Feeder {
	fmt.Println("creating web socket connection to", observerURL, "with timeout", timeout)
	c, _, err := websocket.DefaultDialer.Dial(observerURL, nil)
	if err != nil {
		panic(err)
	}
	fmt.Println("created web socket connection")

	fd := &feeder{
		requestTimeout: timeout,
		readyc:         make(chan struct{}),
		stopc:          make(chan struct{}),
		conn:           c,
	}
	go fd.poll()

	select {
	case <-fd.readyc:
		// wait for initial poll
		fmt.Println("ready")
	case <-time.After(time.Minute):
	}
	return fd
}

func (fd *feeder) poll() {
	fmt.Println("start polling")

	ticker := time.NewTicker(100 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-fd.stopc:
			return
		case <-ticker.C:
		}

		fd.mu.Lock()
		fd.conn.SetWriteDeadline(time.Now().Add(fd.requestTimeout))
		err := fd.conn.WriteMessage(websocket.TextMessage, newRequest())
		fd.mu.Unlock()
		if err != nil {
			fmt.Println("write failed", err)
			continue
		}

		ticker.Reset(0)
		var d []byte
		for {
			fd.mu.Lock()
			fd.conn.SetReadDeadline(time.Now().Add(fd.requestTimeout))
			_, d, err = fd.conn.ReadMessage()
			fd.mu.Unlock()
			if err == nil {
				break
			}

			fmt.Println("read failed", err)
			select {
			case <-fd.stopc:
				return
			case <-ticker.C:
			}
		}

		// passing slice as header to underlying array
		b, err := parse(bytes.NewReader(d))
		if err != nil {
			fmt.Println("failed to parse block", err)
			continue
		}
		rates, rerr := b.getExchangeRates()
		supply, serr := b.getSupply()

		if rerr == nil && len(rates) > 0 {
			select {
			case fd.readyc <- struct{}{}:
			default:
			}
			fd.mu.Lock()
			fd.lastExchangeRates = rates
			fd.mu.Unlock()
		}
		if serr == nil && len(supply) > 0 {
			fd.mu.Lock()
			fd.lastSupply = supply
			fd.mu.Unlock()
		}
	}
}

func (fd *feeder) prices() (ps Prices, err error) {
	fd.mu.RLock()
	rates := fd.lastExchangeRates
	supply := fd.lastSupply
	fd.mu.RUnlock()

	if len(rates) == 0 {
		return Prices{}, errors.New("no rates found")
	}
	if len(supply) == 0 {
		return Prices{}, errors.New("no supply found")
	}

	for denom, amount := range supply {
		rate, ok := rates[denom]
		if !ok {
			fmt.Printf("denom %q rate not found\n", denom)
		}
		ps.Prices = append(ps.Prices, Price{Denom: denom, Price: rate, Volume: amount})
	}
	return ps, nil
}

func (fd *feeder) stop() {
	fd.mu.Lock()
	defer fd.mu.Unlock()

	cerr := fd.conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
	if cerr != nil {
		fmt.Println("write close:", cerr)
	}

	cerr = fd.conn.Close()
	fmt.Println("closed websocket connection", cerr)
}

type request struct {
	Subscribe string `json:"subscribe"`
	ChainID   string `json:"chain_id"`
}

var defaultReq []byte

func init() {
	var err error
	defaultReq, err = json.Marshal(request{Subscribe: "new_block", ChainID: "columbus-4"})
	if err != nil {
		panic(err)
	}
}

func newRequest() []byte {
	return defaultReq
}
