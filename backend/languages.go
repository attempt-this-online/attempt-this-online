package main

import "github.com/vmihailenco/msgpack/v5"

type language struct {
	Name     string `msgpack:"name"`
	Image    string `msgpack:"image"`
	Version  string `msgpack:"version"`
	Url      string `msgpack:"url"`
	Sbcs     bool   `msgpack:"sbcs"`
	SE_class string `msgpack:"SE_class"`
}

var serialisedLanguages []byte

func init() {
	b, err := msgpack.Marshal(languages)
	if err != nil {
		panic(err)
	}
	serialisedLanguages = b
}

var languages = map[string]language{
	"python": {
		Name:     "Python",
		Image:    "attemptthisonline/python",
		Version:  "Latest",
		Url:      "https://www.python.org",
		Sbcs:     false,
		SE_class: "python",
	},
}
