////////////////////////////////////////////////////////////////////////////////
//
// Copyright (c) 2016-2017 MStar Semiconductor, Inc.
// All rights reserved.
//
// Unless otherwise stipulated in writing, any and all information contained
// herein regardless in any format shall remain the sole proprietary of
// MStar Semiconductor Inc. and be kept in strict confidence
// ("MStar Confidential Information") by the recipient.
// Any unauthorized act including without limitation unauthorized disclosure,
// copying, use, reproduction, sale, distribution, modification, disassembling,
// reverse engineering and compiling of the contents of MStar Confidential
// Information is unlawful and strictly prohibited. MStar hereby reserves the
// rights to any and all damages, losses, costs and expenses resulting therefrom.
//
////////////////////////////////////////////////////////////////////////////////

///////////////////////////////////////////////////////////////////////////////////////////////////
/// @file   mstarFb.h
/// @brief  MStar graphic Interface header file
/// @author MStar Semiconductor Inc.
/// @attention
/// <b><em></em></b>
///////////////////////////////////////////////////////////////////////////////////////////////////
#ifndef _UAPI_MSTAR_FB_GRAPHIC_H
#define _UAPI_MSTAR_FB_GRAPHIC_H

#include <linux/ioctl.h>
#include "mi_common_datatype.h"

//-------------------------------------------------------------------------------------------------
//  Type and Structure
//-------------------------------------------------------------------------------------------------
typedef enum
{
    //E_DRV_FB_GOP_COLOR_RGB565
    E_MI_FB_COLOR_FMT_RGB565 = 1,
    //E_DRV_FB_GOP_COLOR_ARGB4444
    E_MI_FB_COLOR_FMT_ARGB4444 = 2,
    //E_DRV_FB_GOP_COLOR_ARGB8888
    E_MI_FB_COLOR_FMT_ARGB8888 = 5,
    //E_DRV_FB_GOP_COLOR_ARGB1555
    E_MI_FB_COLOR_FMT_ARGB1555 = 6,
    //E_DRV_FB_GOP_COLOR_YUV422
    E_MI_FB_COLOR_FMT_YUV422 = 9,
    //E_DRV_FB_GOP_COLOR_I8
    E_MI_FB_COLOR_FMT_I8 = 4,
    //E_DRV_FB_GOP_COLOR_I4
    E_MI_FB_COLOR_FMT_I4 = 13,
    //E_DRV_FB_GOP_COLOR_I2
    E_MI_FB_COLOR_FMT_I2 = 14,
    //E_DRV_FB_GOP_COLOR_INVALID
    E_MI_FB_COLOR_FMT_INVALID  = 12,
}MI_FB_ColorFmt_e;

typedef enum
{
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_DISP_POS = 0x1,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_DISP_SIZE = 0x2,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_BUFFER_SIZE = 0x4,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_SCREEN_SIZE = 0x8,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_PREMUL = 0x10,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_COLOR_FMB = 0x20,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_OUTPUT_COLORSPACE = 0x40,
    E_MI_FB_DISPLAYLAYER_ATTR_MASK_DST_DISP = 0x80,
}MI_FB_DisplayLayerAttrMaskbit_e;

typedef struct MI_FB_GlobalAlpha_s
{
    MI_BOOL bAlphaEnable;	/* alpha enable flag */
    /* alpha channel enable flag  
       TRUE: alpha set to pixel alpha
       FALSE:aplha set to global alpha
    */
    MI_BOOL bAlphaChannel;
    MI_U8 u8Alpha0; /*alpha0 value*/
    MI_U8 u8Alpha1; /*alpha1 value*/
    MI_U8 u8GlobalAlpha;	/* global alpha value */
    MI_U8 u8Reserved;   /* reserved*/
}MI_FB_GlobalAlpha_t;

typedef struct MI_FB_ColorKey_s
{
    MI_BOOL bKeyEnable;
    MI_U8 u8Red;
    MI_U8 u8Green;
    MI_U8 u8Blue;
}MI_FB_ColorKey_t;

typedef struct MI_FB_Rectangle_s
{
    MI_U16 u16Xpos;
    MI_U16 u16Ypos;
    MI_U16 u16Width;
    MI_U16 u16Height;
}MI_FB_Rectangle_t;

typedef enum
{
    //DRV_FB_GOPOUT_RGB
    E_MI_FB_OUTPUT_RGB = 0,
    //DRV_FB_GOPOUT_YUV
    E_MI_FB_OUTPUT_YUV = 1,
}MI_FB_OutputColorSpace_e;

typedef enum
{
    //E_DRV_FB_GOP_DST_IP0
    E_MI_FB_DST_IP0 = 0,
    //E_DRV_FB_GOP_DST_IP0_SUB
    E_MI_FB_DST_IP0_SUB = 1,
    //E_DRV_FB_GOP_DST_MIXER2VE
    E_MI_FB_DST_MIXER2VE = 2,
    //E_DRV_FB_GOP_DST_OP0
    E_MI_FB_DST_OP0 = 3,
    //E_DRV_FB_GOP_DST_VOP
    E_MI_FB_DST_VOP = 4,
    //E_DRV_FB_GOP_DST_IP1
    E_MI_FB_DST_IP1 = 5,
    //E_DRV_FB_GOP_DST_IP1_SUB
    E_MI_FB_DST_IP1_SUB = 6,
    //E_DRV_FB_GOP_DST_MIXER2OP
    E_MI_FB_DST_MIXER2OP = 7,
    //E_DRV_FB_GOP_DST_VOP_SUB
    E_MI_FB_DST_VOP_SUB = 8,
    //E_DRV_FB_GOP_DST_FRC
    E_MI_FB_DST_FRC = 9,
    //E_DRV_FB_GOP_DST_VE
    E_MI_FB_DST_VE = 10,
    //E_DRV_FB_GOP_DST_BYPASS
    E_MI_FB_DST_BYPASS = 11,
    //E_DRV_FB_GOP_DST_OP1
    E_MI_FB_DST_OP1 = 12,
    //E_DRV_FB_GOP_DST_MIXER2OP1
    E_MI_FB_DST_MIXER2OP1 = 13,
    //E_DRV_FB_GOP_DST_DIP
    E_MI_FB_DST_DIP = 14,
    //E_DRV_FB_GOP_DST_GOPScaling
    E_MI_FB_DST_GOPScaling  = 15,
    //E_DRV_FB_GOP_DST_OP_DUAL_RATE
    E_MI_FB_DST_OP_DUAL_RATE = 16,
    //E_DRV_FB_GOP_DST_INVALID
    E_MI_FB_DST_INVALID = 17,
}MI_FB_DstDisplayplane_e;

typedef struct MI_FB_DisplayLayerAttr_s
{
    MI_U32 u32Xpos;    /**the x pos of orign point in screen.Meaning for stretchwindow posx*/
    MI_U32 u32YPos;   /**the y pos of orign point in screen.Meaning for stretchwindow posy*/
    MI_U32 u32dstWidth; /**display buffer dest with in screen.Meaning for stretch window dst width*/
    MI_U32 u32dstHeight; /**display buffer dest hight in screen.Meaning for stretch window dst height*/
    MI_U32 u32DisplayWidth;  /**the width of display buf in fb.Meaning for OSD resolution width */
    MI_U32 u32DisplayHeight;  /**the height of display buf in fb.Meaning for OSD resolution height*/
    MI_U32 u32ScreenWidth;  /**the width of screen.Meaning for timing width.Meaning for timing height*/
    MI_U32 u32ScreenHeight; /** the height of screen */
    MI_BOOL bPreMul;  /**the data drawed in buffer whether is premultiply alpha or not*/
    MI_FB_ColorFmt_e eFbColorFmt; /**the color format of framebuffer*/
    MI_FB_OutputColorSpace_e  eFbOutputColorSpace;  /**output color space*/
    MI_FB_DstDisplayplane_e  eFbDestDisplayPlane;  /**destination displayplane*/
    MI_U32 u32SetAttrMask; /** display attribute modify mask*/
} MI_FB_DisplayLayerAttr_t;

typedef struct MI_FB_CursorImage_s
{
    MI_U32 u32Width; /**width, unit pixel*/
    MI_U32 u32Height; /**Height, unit pixel*/
    MI_U32 u32Pitch; /**Pitch, unit pixel*/
    MI_FB_ColorFmt_e eColorFmt; /**Color format*/
#ifndef __KERNEL__
    const char  *data; /**Image raw data*/
#else
    const char __user *data; /**Image raw data*/
#endif
}MI_FB_CursorImage_t;

typedef enum
{
    E_MI_FB_CURSOR_ATTR_MASK_ICON = 0x1,
    E_MI_FB_CURSOR_ATTR_MASK_POS = 0x2,
    E_MI_FB_CURSOR_ATTR_MASK_ALPHA = 0x4,
    E_MI_FB_CURSOR_ATTR_MASK_SHOW = 0x8,
    E_MI_FB_CURSOR_ATTR_MASK_HIDE = 0x10,
    E_MI_FB_CURSOR_ATTR_MASK_COLORKEY = 0x20,
    E_MI_FB_CURSOR_ATTR_MASK = 0x3F
}MI_FB_CursorAttrMaskbit_e;

typedef struct MI_FB_CursorAttr_s
{
    MI_U32 u32XPos;
    MI_U32 u32YPos;
    MI_U32 u32HotSpotX;
    MI_U32 u32HotSpotY;
    MI_FB_GlobalAlpha_t stAlpha;
    MI_FB_ColorKey_t stColorKey;
    MI_BOOL bShown;
    MI_FB_CursorImage_t stCursorImageInfo;
    MI_U16 u16CursorAttrMask;
}MI_FB_CursorAttr_t;
//-------------------------------------------------------------------------------------------------
//  Macro and Define
//-------------------------------------------------------------------------------------------------
#define FB_IOC_MAGIC 'F'

#define FBIOGET_SCREEN_LOCATION _IOR(FB_IOC_MAGIC, 0x60, MI_FB_Rectangle_t)
#define FBIOSET_SCREEN_LOCATION _IOW(FB_IOC_MAGIC, 0x61, MI_FB_Rectangle_t)

#define FBIOGET_SHOW _IOR(FB_IOC_MAGIC, 0x62, MI_BOOL)
#define FBIOSET_SHOW _IOW(FB_IOC_MAGIC, 0x63, MI_BOOL)

#define FBIOGET_GLOBAL_ALPHA _IOR(FB_IOC_MAGIC, 0x64, MI_FB_GlobalAlpha_t)
#define FBIOSET_GLOBAL_ALPHA _IOW(FB_IOC_MAGIC, 0x65, MI_FB_GlobalAlpha_t)

#define FBIOGET_COLORKEY _IOR(FB_IOC_MAGIC, 0x66, MI_FB_ColorKey_t)
#define FBIOSET_COLORKEY _IOW(FB_IOC_MAGIC, 0x67, MI_FB_ColorKey_t)

#define FBIOGET_DISPLAYLAYER_ATTRIBUTES _IOR(FB_IOC_MAGIC, 0x68, MI_FB_DisplayLayerAttr_t)
#define FBIOSET_DISPLAYLAYER_ATTRIBUTES _IOW(FB_IOC_MAGIC, 0x69, MI_FB_DisplayLayerAttr_t)

#define FBIOGET_CURSOR_ATTRIBUTE _IOR(FB_IOC_MAGIC, 0x70, MI_FB_CursorAttr_t)
#define FBIOSET_CURSOR_ATTRIBUTE _IOW(FB_IOC_MAGIC, 0x71, MI_FB_CursorAttr_t)

#endif