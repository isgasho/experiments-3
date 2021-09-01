package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

type Handler interface {
	start()
}

type handler struct {
	listenerPort uint64
	feeder       Feeder
}

func newHandler(listenerPort uint64, requestTimeout time.Duration, u string) Handler {
	return &handler{
		listenerPort: listenerPort,
		feeder:       newFeeder(requestTimeout, u),
	}
}

func (hd *handler) start() {
	fmt.Println("starting server")

	// start listener
	serverMux := http.NewServeMux()
	serverMux.HandleFunc("/", hd.wrapFunc(handleRequest))

	httpServer := &http.Server{
		Addr:    fmt.Sprintf(":%d", hd.listenerPort),
		Handler: serverMux,
	}

	tch := make(chan os.Signal, 1)
	signal.Notify(tch, syscall.SIGINT)
	done := make(chan struct{})
	go func() {
		fmt.Println("received signal:", <-tch)
		httpServer.Close()
		close(done)
	}()

	fmt.Printf("Serving http://localhost:%d\n", hd.listenerPort)
	if err := httpServer.ListenAndServe(); err != nil {
		fmt.Printf("http server error: %v\n", err)
	}
	select {
	case <-done:
	default:
	}

	hd.feeder.stop()
}

func (hd *handler) wrapFunc(fn func(feeder Feeder, w http.ResponseWriter, req *http.Request)) func(w http.ResponseWriter, req *http.Request) {
	return func(w http.ResponseWriter, req *http.Request) {
		fn(hd.feeder, w, req)
	}
}

func handleRequest(feeder Feeder, w http.ResponseWriter, req *http.Request) {
	switch req.Method {
	case http.MethodGet:
		ps, err := feeder.prices()
		if err != nil {
			fmt.Fprintf(w, "failed to fetch prices %v", err)
			return
		}
		b, err := json.Marshal(ps)
		if err != nil {
			fmt.Fprintf(w, "failed to encode prices %v", err)
			return
		}
		fmt.Fprint(w, string(b))

	default:
		http.Error(w, "Method Not Allowed", 405)
	}
}
