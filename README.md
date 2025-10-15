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

We're providing a logical index on-top of a large scale file system.
Files can be categorized at the logical layer, which either updates the index or moves the files physically, or both.
Additionally, we need to provide automatic file download from external devices onto the large scale device.
One key concern is reliability - all files need to be downloaded automatically as quickly as possible.
We can never "forget" to download a file, or mistakenly not download a duplicate.

## High Level

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
    synced          boolean         // Whether the file has been synced to the target device or not
    lost            boolean         // Whether the file has been lost on the target device or not
    CONSTRAINT unique_path UNIQUE (path) WHERE synced and not lost
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
We'll setup the target device to prohibit/limit writes from other users/programs, but we cannot assume exclusive write access and therefore must build appropriate protections.

The paradigm for `majdool` is that the on-disk state represents the source of truth.


### Flushing from External to Target
The main objective of `majdool` is to safely and reliably write all *novel* files from the external device onto the target device.
This is an at least once semantic.
We do so as follows:

**Drive Flush Procedure**
1. for each file on the external device, compute the hash of `external_file`
2. check `media_index` for this hash (`where hash = hash(external_file) and synced=True`)
    1. If exists, perform content-wise comparison (`external_file` vs `found_from_media_index.path`).
      * If content-wise match: the file already exists on the target, continue.
      * Else (content-wise mismatch): the file does not already exist on the target, use File Flush Procedure.
    2. In absence, use File Flush Procedure.

This procedure reliably transfers all *novel* files from the external to target with the following non-trivial failure modes.
* Step 2 corrupt `media_index` resulting the same file not matching the hash-query.
In this case, the procedure will copy the external file onto the target, resulting in a duplicate.
* Step 2 corrupt `media_index` resulting in a different file matching the hash-query.
In this case, the inner content-wise comparison will catch the difference, and we will still correctly copy the external file onto the target.
* Step 2.1 incorrect content-wise match: in this case, a content-wise match occurs which shouldn't (ex: an exact matching file *accidentally* lives at `found_from_media_index.path`).
Although the `media_index` is incorrect, it is inconsequential with respect to reliably transferring all *novel* files.
The file isn't novel - it exists by definition of this inconsistency condition - and so we don't need to transfer from the external device.
* Step 2.1 incorrect content-wise mismatch: in this case, a content-wise match does not occur which should (ex: the exact matching file has been moved elsewhere or completely deleted).
So the procedure goes ahead copies the external file onto the target, thus reliably transferring all *novel* files.
Indeed, it may be that this file exists twice+ on the target, but we have achieved the at least once semantic.

##### Possible Inconsistencies
* duplicate file: the same file exists twice+ on-disk, with different `$ID`s in the media index
* hash mismatch: file exists in `media_index` and on-disk, but `media_index` contains the wrong hash

**File Flush Procedure**
1. upsert to `media_index` `(hash=content_hash(external_file), path='', synced=False)` -> `$NEW_ID`
2. copy the file onto the target device at `TARGET/flush/$NEW_ID`
3. update `$NEW_ID` in `media_index` with `(path='TARGET/flush/$NEW_ID', synced=True)`

This procedure reliably transfers a single file from the external to the target with the following non-trivial failure modes:
* Step 2 failure (ex: disk out of space): in this case, the state is *consistent* since `path='', synced=False`.
This simply results in `media_index` rows needing garbage collection.
* Step 3 failure (ex: network blip): in this case, the state is *consistent* since `path='', synced=False`.
This results in incomplete indices, but these are not detrimental to the reliability of the Drive Flush Procedure (as explored previously)

##### Possible Inconsistencies
* superfluous row: `media_index` contains un-synced rows which don't reference anything on-disk
* un-synced row: `media_index` contains un-synced rows which do reference a valid file on-disk

### File Moves
We move files by:
1. move the file onto `TARGET/new/location/$ID`
2. update `(path='TARGET/new/location/$ID')`

If this procedure fails at step 2, then we haven't lost the media - it is simply inconsistent with the index.
The flushing section already outlines how this is not detrimental to our system reliability.

##### Possible Inconsistencies
* path mismatch: file exists in `media_index` and on-disk, but `media_index` contains the wrong path

### Consistency Monitor
Although we've solved reliably transferring media, there are various state corruption cases we need to protect against or mitigate.
* disk/content corruption: in this case, the media itself is corrupted (ex: bit flip).
This is solved by use of drive-level redundancy (ex: RAID).
* index/path corruption: in this case, the index becomes inconsistent with the on-disk state (ex: an unsanctioned file move is performed, a partial failure occurs during a sanctioned file move, or a corruption occurs in the `media_index`).
This is solved by running a continuous monitor to detect and fix such inconsistencies.

#### Fixing Inconsistency
Assuming we have a way to detect inconsistencies, let's explore how to fix them.
Notice, we always frame inconsistencies from the perspective of the `media_index` being wrong.
Our monitor does not update the on-disk state - its only job is to reflect the on-disk state into the `media_index`.
This is a safety & simplicity measure, as it makes it easy to audit the monitor to convince ourselves it doesn't lose on-disk data.
We can also restrict the consistency monitor to allow read-only access to the on-disk files.

Also notice, we are only concerned with checking the `synced=True and lost=False` rows from `media_index`.
Anything `synced=False` is not a consistency issue, but rather a garbage collection problem.
Anything `lost=False` may be a data loss issue, but there is no recourse (we cannot magically recreate a lost file).

Inconsistency come in the following forms:
* a) file exists in both places, but with the wrong hash in `media_index`
* b) file exists in both places, but at the wrong path in `media_index`
* c) file is missing from `media_index`
* d) file is superfluous in `media_index`

**A - Fix wrong hash in media_index**
Re-compute the `hash(on_disk_file)` and update the `media_index`.
This will always succeed (there is no constraint preventing multiple cases of the same hash).

**B - Fix wrong path in media_index**
Update the `media_index` with the on-disk path.
Notice, this can fail if another row in the media_index incorrectly uses that path.
Therefore, this should alwasy happen after **D**.

**C - Fix missing from media_index**
Insert into `media_index` with the `hash(on_disk_file)` and on-disk path.
Notice, this can fail if another row in the media_index incorrectly uses that path.
Therefore, this should alwasy happen after **D**.

**D - Fix superfluous in media_index**
Update the `media_index` to the row as `lost`.
This will always succeed (and effectively opens up the path for re-use).

Notice, we must fix these in a specific order - otherwise various constraints may prevent an otherwise valid fix to succeed.
The dependency ordering between fixes is:

```
.
 |- A
 \- D
    |- C
    \- B
```

#### Detecting Inconsistency


```
# on-disk
TARGET/
    dirA/
    dirB/
        ID1.png (dog)
        ID2.png (cat)
        ID3.png (wolf)
        ID4.png (whale)
```

```
# media_index
TARGET/
    dirA/
        ID1.png (dog)           <- wrong path
    dirB/
        ID2.png (cucumber)      <- wrong hash
        ID3.png (wolf)          <- matching
        ID5.png (zebra)         <- superfluous

missing: TARGET/dirB/ID2.png (cat)
```

Let's imagine we have two merkle-trees representing the path only of files.
One represents the files on-disk `M1`, and the other represents the files from the `media_index` `M2`.
Then, we can trivially detect inconsistencies as:
```
delta_x = M1.difference(M2)
delta_y = M2.difference(M1)
skips = set()

# TODO.. case A inconsistencies..
# TODO.. case A inconsistencies..
# TODO.. case A inconsistencies..
# TODO.. case A inconsistencies..

# Fix all case B inconsistencies
for delta in delta_x:
    if delta.id in delta_y.ids():
        skips.add(delta.id)
        # apply fix B for wrong path in media_index

# Fix all case C inconsistencies
for delta in delta_x:
    if delta.id not in skips:
        # apply fix C for missing from media_index

# Fix all case D inconsistencies
for delta in delta_y:
    if delta.id not in skips:
        # apply fix D for superfluous in media_index
```


#### Merkle-tree
```
TARGET/
   .merkle_index := hash((dirA, content(dirA/.merkle_index)), (dirB, content(dirB/.merkle_index)))
   dirA/
     .merkle_index := hash()
   dirB/
     .merkle_index := hash(($ID1.png, content($ID1.png)), ($ID2.mp4, content($ID2.mp4)))
     $ID1.png
     $ID2.mp4
```
This tree represents the on-disk state only.

With an existing merkle-tree representation `M`, we create a new representation `M'` as follows:
1. Recompute an arbitrary directory `d` from `M` by observing its direct children on-disk.
Children which are files are computed by reading the content `hash((file_name, content(file_name)))`.
Children which are directories are computed by reading `.merkle_index`.
2. Compare the computed merkle index for `d` against that from `M`.
If they are the same, then do nothing.
If they are different, then an inconsistency has been found.
3. Upon inconsistency, produce `M'` by recomputing the whole sub-tree under `d` recursively, and then bubbling up to the root.
To be clear, we read the sub-tree at `d` from the disk directly, but between `root` and `d` we reuse the merkle indices from `M - d`.
Then 

To be clear, this is not exhuastive (we're taking depth `d + 2, .., d + N` as correct).
As long as all paths (directories and files) are confirmed in this manner, then the on-disk state is known to be consistent.

Any time an inconsistency is detected, the state must be updated recursively.
1. Compute the state from the sub-tree depth-first upwards.
2. File changes first sync to the `media_index` ()
on-disk `.merkle_index`.
In the case of file inconsistencies, then `media_index` table must also be updated.
It's the job of the consistency monitor to manage this task (ex: periodic random walk, thorough enumeration, etc).

The reason we want the consistency checker to only look at on-disk (and not use the table `media_index`) is:
1. simplified model, and
2. avoid circular reasoning

It's simple to think of the consistency checker as only looking at the source of truth.
Perhaps it reads to find inconsistency and writes back updates to the `media_index`, but this is its *output* - it isn't critical to its functional logic.



DUMP




### Upload Workflow
1. compute hash of `source_file` from external device
2. check `media_index` for this hash
    * In presence, perform content-wise comparison (`source_file` vs `found_from_media_index.path`).
    * In absence:
        1. upsert
        2. copy the file to the target device, then
        2. update the `media_index` (if this fails partway through, it's the consistency checker's job to catch it eventually).




#### Fixing Inconsistency
There are 3 forms of inconsistency, each fixed in their own way:
1. Dangling pointer: this is when the `media_index` references a file that doesn't exist on-disk.
2. Orphan file: this is when a file exists on-disk but is not referenced by the `media_index`.
3.

Specifically, we use a merkle tree to detect inconsistencies between the target device state and update the `media_index`.
Notice, this process does not alter the on-disk state at all.

The merkle tree and consistency monitor work as follows.



--This procedure reliably transfers all *novel* files from the external to target, assuming `media_index` correctly reflects the on-disk state.
--However, let's examine what happens if `media_index` is inconsistent with respect to the target device.


* the same file exists twice+ on-disk
    * it is either missing from the `media_index`, or
    * exists twice+ in the `media_index` (with a different `$ID`)





### Garbage Collection
With the outlined reliable transfer procedure, we are left with the following kinds of data to cleanup:
* duplicate media
* dangling pointers
* partially complete index + file

Solving these are straightforward via async garbage collection process(es).




* file exists in `media_index`, but has a different path on-disk.
* path + file exists on-disk, missing in `media_index`
* path + file exists in `media_index`, missing on-disk
* path + file exists in both, but has different `hash`.


