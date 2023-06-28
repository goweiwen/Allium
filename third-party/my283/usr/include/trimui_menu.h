#ifndef TRIMUSMENU_H
#define TRIMUSMENU_H

#ifdef USE_SDL_H_DIRECT
#include <SDL.h>
#else
#include <SDL/SDL.h>
#endif
#define TRIMUS_MENU

typedef struct _TMenu{    
    //return value >=0 if success
    //return value < 0 if fail    
    
    //fill saved state in statelist, and set savednum in listnm
    int (*getsavedstatelist)(int maxnum, int *statelist);
    //free  what getsavedstatethumb return
    char* (*getsavedstatethumb)(int slot);
    int (*savestate)(int slot);
    int (*loadstate)(int slot);
    void (*quit)();
    int (*togglefullscreen)(int f);
    int (*isfullscreen)();
    int slotnumber;
}TMenu;

int InitResolution(int w, int h);
int ShowMenu(TMenu *menu, SDL_Surface *screen, SDL_Surface *buffer, SDL_Surface *snapshot, const char *title);

#endif