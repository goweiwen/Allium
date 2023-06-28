/*
* cam_os_util_hash.h - Sigmastar
*
* Copyright (C) 2018 Sigmastar Technology Corp.
*
* Author: giggs.huang <giggs.huang@sigmastar.com.tw>
*
* This software is licensed under the terms of the GNU General Public
* License version 2, as published by the Free Software Foundation, and
* may be copied, distributed, and modified under those terms.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
* GNU General Public License for more details.
*
*/


#ifndef __CAM_OS_UTIL_HASH_H__
#define __CAM_OS_UTIL_HASH_H__

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

static inline __attribute__((const))
s32 _CAM_OS_ILOG2_U32(u32 n)
{
    return CAM_OS_FLS(n) - 1;
}

static inline __attribute__((const))
s32 _CAM_OS_ILOG2_U64(u64 n)
{
    return CAM_OS_FLS64(n) - 1;
}

#define CAM_OS_ILOG2(n)                \
(                        \
    __builtin_constant_p(n) ? (        \
        (n) < 1 ? 0 :    \
        (n) & (1ULL << 63) ? 63 : (n) & (1ULL << 62) ? 62 : (n) & (1ULL << 61) ? 61 : (n) & (1ULL << 60) ? 60 : \
        (n) & (1ULL << 59) ? 59 : (n) & (1ULL << 58) ? 58 : (n) & (1ULL << 57) ? 57 : (n) & (1ULL << 56) ? 56 : \
        (n) & (1ULL << 55) ? 55 : (n) & (1ULL << 54) ? 54 : (n) & (1ULL << 53) ? 53 : (n) & (1ULL << 52) ? 52 : \
        (n) & (1ULL << 51) ? 51 : (n) & (1ULL << 50) ? 50 : (n) & (1ULL << 49) ? 49 : (n) & (1ULL << 48) ? 48 : \
        (n) & (1ULL << 47) ? 47 : (n) & (1ULL << 46) ? 46 : (n) & (1ULL << 45) ? 45 : (n) & (1ULL << 44) ? 44 : \
        (n) & (1ULL << 43) ? 43 : (n) & (1ULL << 42) ? 42 : (n) & (1ULL << 41) ? 41 : (n) & (1ULL << 40) ? 40 : \
        (n) & (1ULL << 39) ? 39 : (n) & (1ULL << 38) ? 38 : (n) & (1ULL << 37) ? 37 : (n) & (1ULL << 36) ? 36 : \
        (n) & (1ULL << 35) ? 35 : (n) & (1ULL << 34) ? 34 : (n) & (1ULL << 33) ? 33 : (n) & (1ULL << 32) ? 32 : \
        (n) & (1ULL << 31) ? 31 : (n) & (1ULL << 30) ? 30 : (n) & (1ULL << 29) ? 29 : (n) & (1ULL << 28) ? 28 : \
        (n) & (1ULL << 27) ? 27 : (n) & (1ULL << 26) ? 26 : (n) & (1ULL << 25) ? 25 : (n) & (1ULL << 24) ? 24 : \
        (n) & (1ULL << 23) ? 23 : (n) & (1ULL << 22) ? 22 : (n) & (1ULL << 21) ? 21 : (n) & (1ULL << 20) ? 20 : \
        (n) & (1ULL << 19) ? 19 : (n) & (1ULL << 18) ? 18 : (n) & (1ULL << 17) ? 17 : (n) & (1ULL << 16) ? 16 : \
        (n) & (1ULL << 15) ? 15 : (n) & (1ULL << 14) ? 14 : (n) & (1ULL << 13) ? 13 : (n) & (1ULL << 12) ? 12 : \
        (n) & (1ULL << 11) ? 11 : (n) & (1ULL << 10) ? 10 : (n) & (1ULL <<  9) ?  9 : (n) & (1ULL <<  8) ?  8 : \
        (n) & (1ULL <<  7) ?  7 : (n) & (1ULL <<  6) ?  6 : (n) & (1ULL <<  5) ?  5 : (n) & (1ULL <<  4) ?  4 : \
        (n) & (1ULL <<  3) ?  3 : (n) & (1ULL <<  2) ?  2 : (n) & (1ULL <<  1) ?  1 : (n) & (1ULL <<  0) ?  0 : \
        0) :    \
    (sizeof(n) <= 4) ?      \
    _CAM_OS_ILOG2_U32(n) :  \
    _CAM_OS_ILOG2_U64(n)    \
 )

#define CAM_OS_HASH_SIZE(name) (CAM_OS_ARRAY_SIZE(name))

#define CAM_OS_HASH_BITS(name) CAM_OS_ILOG2(CAM_OS_HASH_SIZE(name))

#define CAM_OS_DEFINE_HASHTABLE(name, bits)                        \
    struct CamOsHListHead_t name[1 << (bits)] =                    \
            { [0 ... ((1 << (bits)) - 1)] = CAM_OS_HLIST_HEAD_INIT }

static inline void _CAM_OS_HASH_INIT(struct CamOsHListHead_t *ht, unsigned int sz)
{
    u32 i;

    for (i = 0; i < sz; i++)
        CAM_OS_INIT_HLIST_HEAD(&ht[i]);
}

#define CAM_OS_HASH_INIT(hashtable) _CAM_OS_HASH_INIT(hashtable, CAM_OS_HASH_SIZE(hashtable))

#define CAM_OS_HASH_ADD(hashtable, node, key)                        \
    CAM_OS_HLIST_ADD_HEAD(node, &hashtable[CAM_OS_HASH_MIN(key, CAM_OS_HASH_BITS(hashtable))])

#define CAM_OS_HASH_FOR_EACH_POSSIBLE(name, obj, member, key)            \
    CAM_OS_HLIST_FOR_EACH_ENTRY(obj, &name[CAM_OS_HASH_MIN(key, CAM_OS_HASH_BITS(name))], member)

static inline void CAM_OS_HASH_DEL(struct CamOsHListNode_t *node)
{
    CAM_OS_HLIST_DEL_INIT(node);
}

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif //__CAM_OS_UTIL_HASH_H__
