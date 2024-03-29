# Usage
## Start uKSMD
```
uksmd &
```
## Add tasks to let uKSMD monitor the crc of the tasks's pages
```
uksmd-ctl add --pid 112

uksmd-ctl add --pid 114
```
## Wait some time to let uKSMD to merge the pages of tasks
```
uksmd-ctl merge
```
## unmerge the pages of a task and let uKSMD doesn't monitor its pages
```
uksmd-ctl del -pid 112
```
