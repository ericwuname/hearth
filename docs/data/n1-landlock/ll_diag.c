// N-1 诊断实测：landlock PATH_BENEATH 对 /dev/null 的 EINVAL 根因矩阵
// C1: /dev/null + FS_RW(全)            -> 真机已证 EINVAL
// C2: /dev/null + 纯文件权限            -> 分辨"目录权限混杂"vs"char device 类型"
// C3: 常规文件 + FS_RW(全)             -> 对照（应成功，目录权限对 reg file？）
// C4: /tmp 目录 + FS_RW               -> 对照（应成功）
#define _GNU_SOURCE
#include <linux/landlock.h>
#include <sys/syscall.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>

#ifndef LANDLOCK_ACCESS_FS_TRUNCATE
#define LANDLOCK_ACCESS_FS_TRUNCATE (1ULL << 14)
#endif
#ifndef SYS_landlock_create_ruleset
#define SYS_landlock_create_ruleset 444
#endif
#ifndef SYS_landlock_add_rule
#define SYS_landlock_add_rule 445
#endif
#ifndef SYS_landlock_restrict_self
#define SYS_landlock_restrict_self 446
#endif

#define LL_EXECUTE   (1ULL << 0)
#define LL_WRITEFILE (1ULL << 1)
#define LL_READFILE  (1ULL << 2)
#define LL_READDIR   (1ULL << 3)
#define LL_REMOVEDIR (1ULL << 4)
#define LL_REMOVEFILE (1ULL << 5)
#define LL_MAKECHAR  (1ULL << 6)
#define LL_MAKEDIR   (1ULL << 7)
#define LL_MAKEREG   (1ULL << 8)
#define LL_TRUNCATE  (1ULL << 14)

#define FS_RW_FULL (LL_EXECUTE | LL_WRITEFILE | LL_READFILE | LL_READDIR | \
                    LL_REMOVEDIR | LL_REMOVEFILE | LL_MAKECHAR | LL_MAKEDIR | \
                    LL_MAKEREG | LL_TRUNCATE)
#define FS_FILE_ONLY (LL_EXECUTE | LL_WRITEFILE | LL_READFILE | LL_TRUNCATE)

static int ll_create_ruleset(unsigned long long handled) {
    struct landlock_ruleset_attr attr = { .handled_access_fs = handled };
    long fd = syscall(SYS_landlock_create_ruleset, &attr, sizeof(attr), 0);
    return (int)fd;
}

static const char *try_add(const char *path, unsigned long long access) {
    int rs = ll_create_ruleset(FS_RW_FULL);
    if (rs < 0) return "create_ruleset_failed";
    int fd = open(path, O_PATH | O_CLOEXEC);
    if (fd < 0) { close(rs); return "open_failed"; }
    struct landlock_path_beneath_attr a = { .allowed_access = access, .parent_fd = fd };
    long r = syscall(SYS_landlock_add_rule, rs, LANDLOCK_RULE_PATH_BENEATH, &a, 0);
    int saved = errno;
    close(fd); close(rs);
    if (r == 0) return "OK";
    static char buf[64];
    snprintf(buf, sizeof(buf), "errno=%d(%s)", saved, strerror(saved));
    return buf;
}

int main(void) {
    // 先造一个常规文件
    FILE *f = fopen("/tmp/ll_reg_file.txt", "w");
    if (f) { fputs("x", f); fclose(f); }

    printf("landlock ABI probe: %s\n", ll_create_ruleset(FS_RW_FULL) >= 0 ? "available" : "unavailable");
    printf("C1 /dev/null  + FS_RW_FULL   -> %s\n", try_add("/dev/null", FS_RW_FULL));
    printf("C2 /dev/null  + FS_FILE_ONLY -> %s\n", try_add("/dev/null", FS_FILE_ONLY));
    printf("C3 regular    + FS_RW_FULL   -> %s\n", try_add("/tmp/ll_reg_file.txt", FS_RW_FULL));
    printf("C3b regular   + FS_FILE_ONLY -> %s\n", try_add("/tmp/ll_reg_file.txt", FS_FILE_ONLY));
    printf("C4 /tmp(dir)  + FS_RW_FULL   -> %s\n", try_add("/tmp", FS_RW_FULL));
    printf("C5 /dev/full  + FS_FILE_ONLY -> %s\n", try_add("/dev/full", FS_FILE_ONLY));
    return 0;
}
