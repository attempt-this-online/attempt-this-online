package main

import (
	"bytes"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
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

func (invocation invocation) invoke(ipHash string) (*result, error) {
	unhashedInvocationId, hashedInvocationId := generateInvocationId()
	dirI := path.Join("/run/ATO_i", hashedInvocationId)
	if err := os.Mkdir(dirI, fs.ModeDir|0755); err != nil {
		log.Println(err)
		return nil, err
	}

	defer func() {
		err := os.RemoveAll(dirI)
		if err != nil {
			// can't really do anything about it other than log
			log.Println("error removing input dir:", err)
		}
	}()

	if err := write(dirI, "code", invocation.Code); err != nil {
		return nil, err
	}
	if err := write(dirI, "input", invocation.Input); err != nil {
		return nil, err
	}
	if err := write(dirI, "arguments", nullTerminate(invocation.Arguments)); err != nil {
		return nil, err
	}
	if err := write(dirI, "options", nullTerminate(invocation.Options)); err != nil {
		return nil, err
	}

	cmd := exec.Command("sudo", "-u", "sandbox", "/usr/local/bin/ATO_sandbox", ipHash, unhashedInvocationId, invocation.Language, strconv.Itoa(invocation.Timeout))
	cmd.Stdin = nil
	cmd.Stdout = os.Stderr
	cmd.Stderr = os.Stderr
	cmd.Env = []string{"PATH=" + os.Getenv("PATH")}

	if err := cmd.Run(); err != nil {
		return nil, err
	}

	dirO := path.Join("/run/ATO_o", hashedInvocationId)
	defer func() {
		err := os.RemoveAll(dirO)
		if err != nil {
			// can't really do anything about it other than log
			log.Println("error removing output dir:", err)
		}
	}()

	var result result

	if encodedStatus, err := os.ReadFile(path.Join(dirO, "status")); err == nil {
		if err = json.Unmarshal(encodedStatus, &result); err != nil {
			return nil, err
		}
	} else {
		return nil, err
	}

	if stderr, err := os.ReadFile(path.Join(dirO, "stderr")); err == nil {
		result.Stderr = stderr
	} else {
		return nil, err
	}
	if stdout, err := os.ReadFile(path.Join(dirO, "stdout")); err == nil {
		result.Stdout = stdout
	} else {
		return nil, err
	}

	return &result, nil
}
