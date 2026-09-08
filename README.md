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

## What it normalizes

- Runs of spaces or tabs between fields collapse to a single space.
- Whitespace inside a field, such as `1, 3, 5`, collapses to `1,3,5`.
- Named months and weekdays fold to one case: `MON`, `mon`, and `Mon` all
  become `Mon`.
- Ranges and step values (`mon-fri`, `*/15`, `0-6/2`) keep their structure
  but get the same whitespace and case treatment as plain fields.

## What it does not do (yet)

It does not validate that field values fall within a legal range for
their position, and it only understands the standard 5-field form
(minute hour day-of-month month day-of-week), not the 6-field form with
seconds. See the roadmap in the project notes for what's planned next.

## License

MIT, see LICENSE.
