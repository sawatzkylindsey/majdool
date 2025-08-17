# majdool
media transfer and cataloguing

# Usage

### Setup
```
$ brew install postgresql
$ psql postgres

postgres=# CREATE DATABASE majdool;
```

###
```
$ cargo build
$ ./target/debug/majdool
```

# Design

We're providing a logical index ontop of a large scale file system.
Files can be categorized at the logical layer, which either updates the index or moves the files physically, or both.
Additionally, we need to provide automatic file download from external devices onto the large scale device.
One key concern is reliability - all files need to be downloaded automatically as quickly as possible.
We can never "forget" to download a file, or mistakenly not download a duplicate.

### High Level

We'll write files directly into the physical large scale file system, while also maintaining a logical index in a database.

```
TARGET/
   dir1/
   dir2/
     $ID.png
     $ID.mp4
```

```
TABLE media_index (
    id              PK              // Uniquely identifies the file
    hash            bytes           // Hash of the media - may not be unique in case of collisions
    path            text            // Physical location of the media on the target device
)

TABLE labels (
    id              PK
    media_index_id  FK
    ..
)
```

Let's not focus too much on the categorization system (ex: labels table) at this point.
The main idea is we'll store a row per piece of media in the index, and we'll use a hash to ~almost uniquely identify these.
In any case where the hash already exists, we need to perform a content level check to see whether the file is the same or not.
Notice, we only index files on the target device - files in the external storages devices which we'll download from are not included in this index.

It's not detailed in the schedule, but we'll also maintain a "merkle tree" index across the directories of the file system.
This is primarily to afford mutation detection/correction.
We'll setup the system to prohibit/limit external writes as much as possible, but nevertheless detecting unknown changes will be important.




