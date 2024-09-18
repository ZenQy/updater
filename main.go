package main

import (
	"fmt"
	"os"
	"regexp"
	"strings"

	"github.com/go-resty/resty/v2"
	"github.com/tidwall/gjson"
	"gopkg.in/yaml.v3"
)

type Conf struct {
	Type     string `yaml:"type"`
	Name     string `yaml:"name"`
	Perfix   string `yaml:"perfix,omitempty"`
	Version  string `yaml:"version,omitempty"`
	Filename string `yaml:"filename,omitempty"`
}

func main() {
	var conf []Conf
	b, _ := os.ReadFile("config.yaml")
	yaml.Unmarshal(b, &conf)

	msg := ""

	for i, c := range conf {
		switch c.Type {
		case "gamedva":
			{
				Gamedva(&c)
				if conf[i].Version != c.Version {
					conf[i] = c
					msg += fmt.Sprintf("\n[🔗%s(%s)](https://gamedva.com/%s)", c.Name, c.Version, c.Name)
				}
			}
		case "github":
			{
				Github(&c)
				if conf[i].Version != c.Version {
					conf[i] = c
					msg += fmt.Sprintf("\n[🔗%s(%s)](https://github.com/%s/releases/download/%s/%s)",
						c.Name[strings.Index(c.Name, "/")+1:],
						c.Version, c.Name, c.Version,
						strings.Replace(c.Filename, "$ver", c.Version, -1))
				}
			}
		}
	}

	if msg != "" {
		b, err := yaml.Marshal(&conf)
		if err != nil {
			os.Exit(-1) // 序列化失败
		}
		os.WriteFile("config.yaml", b, 0o644)

		Send("以下软件需要更新：" + msg)
	}
}

// Send 发送到Telegram
func Send(text string) {
	token := os.Getenv("TELEGRAM_TOKEN")
	chatID := os.Getenv("TELEGRAM_TO")

	url := "https://api.telegram.org/bot" + token + "/sendMessage"
	body := `{"chat_id":"` + chatID + `","text":"` + text + `","parse_mode":"Markdown","link_preview_options":{"is_disabled":true}}`

	resty.New().R().
		SetHeader("Content-Type", "application/json").
		SetBody(body).Post(url)
}

func Gamedva(c *Conf) {
	client := resty.New()
	resp, err := client.R().Get("https://gamedva.com/" + c.Name)
	if err != nil {
		os.Exit(-1) // 查询失败
	}
	r := regexp.MustCompile(`<strong>Version</td><td>(.*?)</td>`)
	m := r.FindStringSubmatch(resp.String())
	if len(m) != 2 {
		os.Exit(-1) // 查询失败
	}
	c.Version = m[1]
}

func Github(c *Conf) {
	client := resty.New()
	resp, err := client.R().Get("https://api.github.com/repos/" + c.Name + "/releases/latest")
	if err != nil {
		os.Exit(-1) // 查询失败
	}

	c.Version = strings.TrimPrefix(gjson.Get(resp.String(), "tag_name").String(), c.Perfix)
}
