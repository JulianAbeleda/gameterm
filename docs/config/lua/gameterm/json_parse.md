---
title: gameterm.json_parse
tags:
 - utility
 - json
---


# `gameterm.json_parse(string)`

{{since('20220807-113146-c2fee766')}}

Parses the supplied string as json and returns the equivalent lua values:

```
> gameterm.json_parse('{"foo":"bar"}')
{
    "foo": "bar",
}
```
