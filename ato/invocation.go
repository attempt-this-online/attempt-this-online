package ato

import (
	"bytes"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"io"
	"io/fs"
	"log"
	"os"
	"os/exec"
	"path"
	"strconv"
)

type invocation struct {
	Language  string   `msgpack:"language"`
	Code      []byte   `msgpack:"code"`
	Input     []byte   `msgpack:"input"`
	Arguments [][]byte `msgpack:"arguments"`
	Options   [][]byte `msgpack:"options"`
	Timeout   int      `msgpack:"timeout"`
}

func write(dir string, name string, data []byte) error {
	if file, err := os.Create(path.Join(dir, name)); err == nil {
		defer file.Close()
		file.Write(data)
		return nil
	} else {
		log.Println(err)
		return err
	}
}

func generateInvocationId() (string, string) {
	const size = 16
	buf := make([]byte, size)
	if _, err := rand.Read( /* write random data to */ buf); err != nil {
		log.Panic("random generation failed", err)
	}
	hexId := hex.EncodeToString(buf)
	hexIdHashed := sha256.Sum256([]byte(hexId))
	hexIdHashedHex := hex.EncodeToString(hexIdHashed[:])
	return hexId, hexIdHashedHex
}

func nullTerminate(args [][]byte) []byte {
	var buf bytes.Buffer
	for _, arg := range args {
		buf.Write(arg)
		buf.WriteByte(0)
	}
	return buf.Bytes()
}

type result struct {
	Stdout          []byte `json:"-" msgpack:"stdout"`
	Stderr          []byte `json:"-" msgpack:"stderr"`
	StdoutTruncated bool   `json:"-" msgpack:"stdout_truncated"`
	StderrTruncated bool   `json:"-" msgpack:"stderr_truncated"`
	StatusType      string `json:"status_type" msgpack:"status_type"`
	StatusValue     int    `json:"status_value" msgpack:"status_value"`
	TimedOut        bool   `json:"timed_out" msgpack:"timed_out"`
	Real            int64  `json:"real" msgpack:"real"`
	Kernel          int64  `json:"kernel" msgpack:"kernel"`
	User            int64  `json:"user" msgpack:"user"`
	MaxMem          int64  `json:"max_mem" msgpack:"max_mem"`
	Waits           int64  `json:"waits" msgpack:"waits"`
	Preemptions     int64  `json:"preemptions" msgpack:"preemptions"`
	MajorPageFaults int64  `json:"minor_page_faults" msgpack:"minor_page_faults"`
	MinorPageFaults int64  `json:"major_page_faults" msgpack:"major_page_faults"`
	InputOps        int64  `json:"input_ops" msgpack:"input_ops"`
	OutputOps       int64  `json:"output_ops" msgpack:"output_ops"`
}

func (invocation invocation) invoke() (*result, error) {
	unhashedInvocationId, hashedInvocationId := generateInvocationId()
	dir := path.Join("/run/ATO", hashedInvocationId)
	if err := os.Mkdir(dir, fs.ModeDir|0755); err != nil {
		log.Println(err)
		return nil, err
	}

	defer func() {
		err := os.RemoveAll(dir)
		if err != nil {
			// can't really do anything about it other than log
			log.Println("error removing input dir:", err)
		}
	}()

	if err := write(dir, "code", invocation.Code); err != nil {
		return nil, err
	}
	if err := write(dir, "input", invocation.Input); err != nil {
		return nil, err
	}
	if err := write(dir, "arguments", nullTerminate(invocation.Arguments)); err != nil {
		return nil, err
	}
	if err := write(dir, "options", nullTerminate(invocation.Options)); err != nil {
		return nil, err
	}

	cmd := exec.Command(
		"/usr/local/bin/ATO_sandbox",
		unhashedInvocationId,
		invocation.Language,
		strconv.Itoa(invocation.Timeout),
		Languages[invocation.Language].Image,
	)
	cmd.Env = []string{"PATH=" + os.Getenv("PATH")}
	cmd.Stdin = nil

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, err
	}
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return nil, err
	}

	if err := cmd.Start(); err != nil {
		return nil, err
	}

	var result result
	wait := make(chan error)
	go func() {
		lr := io.LimitedReader{
			R: stdout,
			N: 128 * 1024,
		}
		var err error
		result.Stdout, err = io.ReadAll(&lr)
		if lr.N <= 0 {
			result.StdoutTruncated = true
		}
		err2 := stdout.Close()
		if err == nil {
			err = err2
		}
		wait <- err
	}()
	go func() {
		lr := io.LimitedReader{
			R: stderr,
			N: 32 * 1024,
		}
		var err error
		result.Stderr, err = io.ReadAll(&lr)
		if lr.N <= 0 {
			result.StderrTruncated = true
		}
		err2 := stderr.Close()
		if err == nil {
			err = err2
		}
		wait <- err
	}()

	for i := 0; i < 2; i++ {
		err := <-wait
		if err != nil {
			return nil, err
		}
	}

	if encodedStatus, err := os.ReadFile(path.Join(dir, "status")); err == nil {
		if err = json.Unmarshal(encodedStatus, &result); err != nil {
			return nil, err
		}
	} else {
		return nil, err
	}

	return &result, nil
}
