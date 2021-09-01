package main

import (
	"flag"
	"time"
)

func main() {
	listenerPort := flag.Uint64("listener-port", 8080, "listener port")
	requestTimeoutSeconds := flag.Uint64("request-timeout-seconds", 10, "request timeout in seconds")
	observerURL := flag.String("observer-url", "wss://observer.terra.dev", "observer URL")
	flag.Parse()

	srv := newHandler(*listenerPort, time.Duration(*requestTimeoutSeconds)*time.Second, *observerURL)
	srv.start()
}
