package ato

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
	b, err := msgpack.Marshal(Languages)
	if err != nil {
		panic(err)
	}
	serialisedLanguages = b
}

var Languages = map[string]language{
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
		Image:    "attemptthisonline/python_with_common_libraries",
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
	"bash": {
		Name:    "Bash",
		Image:   "attemptthisonline/bash",
		Version: "Latest",
		Url:     "https://www.gnu.org/software/bash/",
		Sbcs:    false,
	},
	"pip": {
		Name:    "Pip",
		Image:   "attemptthisonline/pip",
		Version: "Latest",
		Url:     "https://github.com/dloscutoff/pip",
		Sbcs:    false,
	},
	"funky2": {
		Name:    "Funky2",
		Image:   "attemptthisonline/funky2",
		Version: "Latest",
		Url:     "https://funky2.a-ta.co/",
		Sbcs:    false,
	},
	"c_gcc": {
		Name:    "C (GCC)",
		Image:   "attemptthisonline/base",
		Version: "11",
		Url:     "https://gcc.gnu.org",
		Sbcs:    false,
	},
}
