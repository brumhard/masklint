# Subcommand Testing

The following maskfile demonstrates the ability to parse nested subcommands.

## grandparent

### grandparent parent

#### grandparent parent command

```bash
mkdir $unset
```

### grandparent parent command2

```bash
files="file1.txt file2.txt"
rm $files
```

