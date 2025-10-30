## Speaker notes

Starting on version 0.10.0, _presenterm_ allows presentations to define speaker notes. The way this works is:

* You start an instance of _presenterm_ using the `--publish-speaker-notes` parameter. This will be the main instance in 
which you will present like you usually do.
* Another instance should be started using the `--listen-speaker-notes` parameter. This instance will only display 
speaker notes in the presentation and will automatically change slides whenever the main instance does so.
* Optionally, you can start another instance using the `--mirror-main-slide` parameter. This instance will display the
main slide content (what the audience sees) and will automatically follow the main instance. This is useful when the
speaker cannot directly see the projected screen.

For example:

```bash
# Start the main instance
presenterm demo.md --publish-speaker-notes

# In another shell: start the speaker notes instance
presenterm demo.md --listen-speaker-notes

# Optionally, in a third shell: mirror the main slide for the speaker
presenterm demo.md --mirror-main-slide
```

[![asciicast](https://asciinema.org/a/ETusvlmHuHrcLKzwa0CMQRX2J.svg)](https://asciinema.org/a/ETusvlmHuHrcLKzwa0CMQRX2J)

See the [speaker notes example](https://github.com/mfontanini/presenterm/blob/master/examples/speaker-notes.md) for more 
information.

### Defining speaker notes

In order to define speaker notes you can use the `speaker_notes` comment command:

```markdown
Normal text

<!-- speaker_note: this is a speaker note -->

More text
```

When running this two instance setup, the main one will show "normal text" and "more text", whereas the second one will 
only show "this is a speaker note" on that slide.

### Multiline speaker notes

You can use multiline speaker notes by using the appropriate YAML syntax:

```yaml
<!-- 
speaker_note: |
  something
  something else
-->
```

### Multiple instances

On Linux and Windows, you can run multiple instances in publish mode and multiple instances in listen mode at the same 
time. Each instance will only listen to events for the presentation it was started on.

On Mac this is not supported and only a single listener can be used at a time.

### Enabling publishing by default

You can use the `speaker_notes.always_publish` key in your config file to always publish speaker notes. This means you 
will only ever need to use `--listen-speaker-notes` and you will never need to use `--publish-speaker-notes`:

```yaml
speaker_notes:
  always_publish: true
```

### Internals

This uses UDP sockets on localhost to communicate between instances. The main instance sends events every time a slide 
is shown and the listener instances listen to them and displays the speaker notes for that specific slide.
