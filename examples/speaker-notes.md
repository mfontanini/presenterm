Speaker Notes
===

`presenterm` supports speaker notes.

You can use the following HTML comment throughout your presentation markdown file:

```markdown
<!-- speaker_note: Your speaker note goes here. -->
```

<!-- speaker_note: This is a speaker note from slide 1. -->

And you can run a separate instance of `presenterm` to view them.

<!-- speaker_note: You can use multiple speaker notes within each slide and interleave them with other markdown. -->

<!-- end_slide -->

Usage
===
Run the following two commands in separate terminals.

<!-- speaker_note: This is a speaker note from slide 2. -->

The `--publish-speaker-notes` argument will render your actual presentation as normal, without speaker notes:

```
presenterm --publish-speaker-notes examples/speaker-notes.md
```

The `--listen-speaker-notes` argument will render only the speaker notes for the current slide being shown in the actual 
presentation:

```
presenterm --listen-speaker-notes examples/speaker-notes.md
```

<!-- speaker_note: Demonstrate changing slides in the actual presentation. -->

As you change slides in your actual presentation, the speaker notes presentation slide will automatically navigate to the correct slide.

<!-- speaker_note: Isn't that cool? -->
