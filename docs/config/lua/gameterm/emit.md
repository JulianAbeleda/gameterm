---
title: gameterm.emit
tags:
 - event
---

# `gameterm.emit(event_name, args...)`

{{since('20201031-154415-9614e117')}}

`gameterm.emit` resolves the registered callback(s) for the specified
event name and calls each of them in turn, passing the additional
arguments through to the callback.

If a callback returns `false` then it prevents later callbacks from
being called for this particular call to `gameterm.emit`, and `gameterm.emit`
will return `false` to indicate that no additional/default processing
should take place.

If none of the callbacks returned `false` then `gameterm.emit` will
itself return `true` to indicate that default processing should take
place.

This function has no special knowledge of which events are defined by
gameterm, or what their required arguments might be.

See [gameterm.on](on.md) for more information about event handling.

