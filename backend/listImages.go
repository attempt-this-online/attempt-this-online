package main

import (
    "fmt"

    "github.com/attempt-this-online/attempt-this-online/backend"
)

func main() {
    images := make(map[string]struct{})
    for _, language := range backend.Languages {
        images[language.Image] = struct{}{}
    }
    for image := range images {
        fmt.Println(image)
    }
    return
}
