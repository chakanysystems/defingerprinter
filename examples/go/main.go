package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
)

func main() {
	request, err := json.Marshal(map[string]string{
		"op": "redact", "language": "en", "text": "Contact alice@example.com",
	})
	if err != nil {
		log.Fatal(err)
	}
	cmd := exec.Command(os.Args[1])
	cmd.Stdin = bytes.NewReader(append(request, '\n'))
	output, err := cmd.Output()
	if err != nil {
		log.Fatal(err)
	}
	var response struct {
		OK     bool   `json:"ok"`
		Text   string `json:"text"`
		Reason string `json:"reason"`
	}
	if err := json.Unmarshal(output, &response); err != nil {
		log.Fatal(err)
	}
	if !response.OK {
		log.Fatal(response.Reason)
	}
	fmt.Println(response.Text) // Contact [EMAIL_1]
}
