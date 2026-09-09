// O-3 取证：目录创建 syscall 逐一探测（沙箱外 vs 沙箱内对比）
#define _GNU_SOURCE
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <errno.h>
#include <string.h>
int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    long r;
    r = syscall(SYS_mkdir, "/tmp/probe_d1", 0755);
    printf("mkdir(83)          -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_mkdirat, AT_FDCWD, "/tmp/probe_d2", 0755);
    printf("mkdirat(258)       -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_openat, AT_FDCWD, "/tmp/probe_f1", O_CREAT|O_WRONLY, 0644);
    printf("openat(257)        -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_getdents64, 0, NULL, 0);
    printf("getdents64(217)    -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_chmod, "/tmp/probe_f1", 0644);
    printf("chmod(90)          -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_fchmodat, AT_FDCWD, "/tmp/probe_f1", 0644, 0);
    printf("fchmodat(268)      -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_rmdir, "/tmp/probe_d2");
    printf("rmdir(84)          -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok"); errno=0;
    r = syscall(SYS_unlinkat, AT_FDCWD, "/tmp/probe_f1", 0);
    printf("unlinkat(263)      -> %ld errno=%d(%s)\n", r, errno, r<0?strerror(errno):"ok");
    return 0;
}
