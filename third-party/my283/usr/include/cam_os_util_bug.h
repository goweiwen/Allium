/*
* cam_os_util_bug.h - Sigmastar
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


#ifndef __CAM_OS_UTIL_BUG_H__
#define __CAM_OS_UTIL_BUG_H__

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

#define CAM_OS_BUG() do { \
	CamOsPrintf("BUG: failure at %s:%d/%s()!\n", __FILE__, __LINE__, __func__); \
	CamOsPanic("BUG!"); \
} while (0)

#define CAM_OS_BUG_ON(condition) do { if (unlikely(condition)) CAM_OS_BUG(); } while (0)

#define CAM_OS_MAX_ERRNO   4096
#define CAM_OS_IS_ERR_VALUE(x) CAM_OS_UNLIKELY((x) >= (unsigned long)-CAM_OS_MAX_ERRNO)

#define CAM_OS_ERR_PTR(x) (void *)(x)

#define CAM_OS_PTR_ERR(x) (long)(x)

#define CAM_OS_IS_ERR(x) CAM_OS_IS_ERR_VALUE((unsigned long)(x))

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif //__CAM_OS_UTIL_BUG_H__
