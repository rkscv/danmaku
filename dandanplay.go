package main

import (
	"bytes"
	"context"
	"crypto/md5"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path"
	"slices"
	"strconv"
	"strings"

	"github.com/rivo/uniseg"
)

type dandanplayMatch struct {
	IsMatched bool `json:"isMatched"`
	Matches   []struct {
		EpisodeID int `json:"episodeId"`
	} `json:"matches"`
}

type dandanplayComment struct {
	Count    int `json:"count"`
	Comments []struct {
		Cid int    `json:"cid"`
		P   string `json:"p"`
		M   string `json:"m"`
	} `json:"comments"`
}

func dandanplayComments(ctx context.Context, name string) ([]danmaku, error) {
	f, err := os.Open(name)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	h := md5.New()
	// https://api.dandanplay.net/swagger/ui/index
	if _, err = io.CopyN(h, f, 16*1024*1024); err != nil {
		return nil, err
	}
	b, err := json.Marshal(map[string]string{
		"fileName": path.Base(name),
		"fileHash": fmt.Sprintf("%x", h.Sum(nil)),
	})
	if err != nil {
		panic(err)
	}
	req, err := http.NewRequestWithContext(ctx, "POST", "https://api.dandanplay.net/api/v2/match", bytes.NewReader(b))
	if err != nil {
		panic(err)
	}
	req.Header.Add("Content-Type", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	d := json.NewDecoder(resp.Body)
	var match dandanplayMatch
	if err = d.Decode(&match); err != nil {
		panic(err)
	}
	if len(match.Matches) > 1 {
		return nil, errors.New("multiple matching episodes")
	} else if !match.IsMatched {
		return nil, errors.New("no matching episode")
	}

	url := fmt.Sprintf("https://api.dandanplay.net/api/v2/comment/%d?withRelated=true", match.Matches[0].EpisodeID)
	req, err = http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		panic(err)
	}
	resp, err = http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	d = json.NewDecoder(resp.Body)
	var comment dandanplayComment
	if err = d.Decode(&comment); err != nil {
		panic(err)
	}
	comments := make([]danmaku, comment.Count)
	for i, comment := range comment.Comments {
		p := strings.SplitN(comment.P, ",", 4)
		t, err := strconv.ParseFloat(p[0], 64)
		if err != nil {
			panic(err)
		}
		c, err := strconv.Atoi(p[2])
		if err != nil {
			panic(err)
		}
		comments[i] = danmaku{
			Message:              strings.ReplaceAll(comment.M, "\n", "\\N"),
			GraphemeClusterCount: uniseg.GraphemeClusterCount(comment.M),
			Time:                 t,
			R:                    uint8(c / (256 * 256)),
			G:                    uint8(c % (256 * 256) / 256),
			B:                    uint8(c % 256),
			X:                    INVALID_X,
			Y:                    INVALID_Y,
		}
	}
	slices.SortFunc(comments, func(a, b danmaku) int {
		return int(a.Time - b.Time)
	})
	return comments, nil
}
