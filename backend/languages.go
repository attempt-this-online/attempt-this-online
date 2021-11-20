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
	"whython": {
		Name:     "Whython",
		Image:    "attemptthisonline/whython",
		Version:  "Latest",
		Url:      "https://github.com/pxeger/whython",
		Sbcs:     false,
		SE_class: "python",
	},
	"python": {
		Name:     "Python",
		Image:    "attemptthisonline/python",
		Version:  "Latest",
		Url:      "https://www.python.org",
		Sbcs:     false,
		SE_class: "python",
	},
	"zsh": {
		Name:     "Zsh",
		Image:    "attemptthisonline/zsh",
		Version:  "5",
		Url:      "https://www.zsh.org/",
		Sbcs:     false,
		SE_class: "bash",
	},
	"jelly": {
		Name:    "Jelly",
		Image:   "attemptthisonline/jelly",
		Version: "70c9fd93",
		Url:     "https://github.com/DennisMitchell/jellylanguage",
		Sbcs:    true,
	},
	"ruby": {
		Name:     "Ruby",
		Image:    "attemptthisonline/ruby",
		Version:  "Latest",
		Url:      "https://www.ruby-lang.org/",
		Sbcs:     false,
		SE_class: "ruby",
	},
	"python2": {
		Name:     "Python 2",
		Image:    "attemptthisonline/python2",
		Version:  "2",
		Url:      "https://docs.python.org/2/",
		Sbcs:     false,
		SE_class: "python2",
	},
	"scala3": {
		Name:    "Scala 3",
		Image:   "attemptthisonline/scala3",
		Version: "3",
		Url:     "https://www.scala-lang.org/",
		Sbcs:    false,
	},
	"scala2": {
		Name:    "Scala 2",
		Image:   "attemptthisonline/scala2",
		Version: "2",
		Url:     "https://www.scala-lang.org/",
		Sbcs:    false,
	},
	"java": {
		Name:     "Java",
		Image:    "attemptthisonline/java",
		Version:  "Latest",
		Url:      "https://en.wikipedia.org/wiki/Java_(programming_language)",
		Sbcs:     false,
		SE_class: "java",
	},
	"tictac": {
		Name:    "Tictac",
		Image:   "attemptthisonline/tictac",
		Version: "Latest",
		Url:     "https://github.com/pxeger/tictac",
		Sbcs:    true,
	},
}
