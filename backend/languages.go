package main

type language struct {
	name     string
	image    string
	version  string
	url      string
	sbcs     bool
	SE_class string
}

var languages = map[string]language{
	"python": {
		"Python",
		"attemptthisonline/python",
		"version",
		"https://www.python.org",
		false,
		"python",
	},
}
