# cronfmt

Cron expressions accumulate inconsistency: some lines use tabs, some use
runs of spaces, some write `MON-FRI` and others `mon-fri`, some put no
space after a comma. None of that changes what the schedule means, but it
makes crontabs annoying to diff and review. `cronfmt` reads cron lines and
rewrites each one into a single canonical form, leaving comments, blank
lines, and the trailing command untouched.

It reads one line at a time and writes the normalized line immediately, so
a large crontab or a piped stream of schedule strings never gets buffered
into memory as a whole.

## Usage

From a file:

```
cargo run -- crontab.txt
```

From a pipe:

```
cat crontab.txt | cargo run
```

Example input:

```
*  *\t* *   *   /usr/bin/backup.sh
1, 3,5    0-6/2 * MON-FRI  *
# nightly cleanup
```

Output:

```
* * * * * /usr/bin/backup.sh
1,3,5 0-6/2 * Mon-Fri *
# nightly cleanup
```

Lines with fewer than 5 schedule fields are passed through unchanged, with
a warning printed to stderr and a nonzero exit code at the end of the run.

## Check mode

```
cargo run -- --check crontab.txt
```

`--check` reports whether a file is already in canonical form instead of
printing the rewritten lines. It writes nothing to stdout; for each line
that would change it prints `line N would be reformatted` to stderr, and
the process exits nonzero if any line needed reformatting or failed to
parse. This is meant for CI: run it against a crontab and fail the build
if someone committed an unnormalized schedule.

## What it normalizes

- Runs of spaces or tabs between fields collapse to a single space.
- Whitespace inside a field, such as `1, 3, 5`, collapses to `1,3,5`.
- Named months and weekdays fold to one case: `MON`, `mon`, and `Mon` all
  become `Mon`.
- Ranges and step values (`mon-fri`, `*/15`, `0-6/2`) keep their structure
  but get the same whitespace and case treatment as plain fields.

Each field is also checked against the legal range for its position:
second 0-59, minute 0-59, hour 0-23, day-of-month 1-31, month 1-12 (or
`Jan`-`Dec`), and day-of-week 0-7 (or `Sun`-`Sat`). A value out of range,
a name used in the wrong field (`Mon` in the month position), an unknown
name, or a step that isn't a positive integer is reported as an error on
stderr and the offending line is passed through unchanged so nothing is
silently dropped.

## Special strings

A schedule can be one of the `@`-prefixed shorthands instead of a field
list: `@reboot`, `@yearly`, `@annually`, `@monthly`, `@weekly`, `@daily`,
`@midnight`, or `@hourly`. These are recognized by their leading `@` and
normalized to lowercase (`@REBOOT` becomes `@reboot`); the rest of the
line, if any, is treated as the command and left alone. `@annually` and
`@yearly` mean the same thing, as do `@midnight` and `@daily`, but each
keeps its own spelling rather than being folded into the other - the goal
is consistent casing, not rewriting which alias someone chose. An
unrecognized `@word` is reported as an error and passed through unchanged,
the same as an invalid field.

## Field counts

The standard form is 5 fields: minute hour day-of-month month
day-of-week. Some cron variants add a leading seconds field, making 6.
A line has no marker saying which form it uses, so when it has 6 or more
tokens, `cronfmt` first tries reading the first 6 as a schedule with
seconds; if that doesn't check out (a value out of an allowed range, an
unknown name, and so on) it falls back to the standard 5-field reading.
This means a 5-field line whose command happens to look like a valid
6th schedule field can be misread as having seconds - rare in practice,
but worth knowing about.

## License

MIT, see LICENSE.
