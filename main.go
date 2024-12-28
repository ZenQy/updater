package main

import (
	"errors"
	"fmt"
	"log/slog"
	"os"
	"regexp"

	"github.com/go-resty/resty/v2"
	"github.com/tidwall/gjson"
	"gopkg.in/yaml.v3"
)

type Conf struct {
	Type    string `yaml:"type"`
	Name    string `yaml:"name"`
	Version string `yaml:"version,omitempty"`
	URL     string `yaml:"url,omitempty"`
}

func main() {
	var conf []Conf
	b, err := os.ReadFile("config.yaml")
	if err != nil {
		slog.Error("读取配置文件失败：", "err", err)
		os.Exit(-1)
	}
	if err := yaml.Unmarshal(b, &conf); err != nil {
		slog.Error("解析配置文件失败：", "err", err)
		os.Exit(-1)
	}

	msg := ""

	for i, c := range conf {
		switch c.Type {
		case "liteapks":
			{
				err = Liteapks(&c)
				if err != nil {
					slog.Error("Liteapks", "error:", err)
					break
				}
				if conf[i].Version != c.Version {
					conf[i] = c
					msg += fmt.Sprintf("\n[🔗%s(%s)](%s)", c.Name, c.Version, c.URL)
				}
			}
		case "github":
			{
				err = Github(&c)
				if err != nil {
					slog.Error("Github", "error:", err)
					break
				}
				if conf[i].Version != c.Version {
					conf[i] = c
					msg += fmt.Sprintf("\n[🔗%s(%s)](%s)", c.Name, c.Version, c.URL)
				}
			}
		}
	}

	if msg != "" {
		b, err := yaml.Marshal(&conf)
		if err != nil {
			slog.Error("序列化config.yaml出错")
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

func Liteapks(c *Conf) error {
	client := resty.New()
	resp, err := client.R().Get("https://liteapks.com/" + c.Name + ".html")
	if err != nil {
		return err
	}

	txt := resp.String()
	{
		r := regexp.MustCompile(`"softwareVersion": "(.*?) ?",`)
		m := r.FindStringSubmatch(txt)
		if len(m) != 2 {
			return errors.New("正则查询版本号失败")
		}
		c.Version = m[1]
	}
	{
		r := regexp.MustCompile(`href="(https://liteapks.com/download/` + c.Name + `-\d+?)"`)
		m := r.FindStringSubmatch(txt)
		if len(m) != 2 {
			return errors.New("正则查询下载页面失败")
		}
		c.URL = m[1]
	}

	return nil
}

func Github(c *Conf) error {
	client := resty.New()
	resp, err := client.R().Get("https://api.github.com/repos/" + c.Name + "/releases/latest")
	if err != nil {
		return err
	}

	c.Version = gjson.Get(resp.String(), "tag_name").String()
	c.URL = "https://github.com/" + c.Name + "/releases/tag/" + c.Version
	return nil
}
