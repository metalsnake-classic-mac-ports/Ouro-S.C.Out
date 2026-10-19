//
//  main.mm
//  S.C.Out
//
//  Created by Paul MERCAT on 02/05/08.
//  Copyright 2008. All rights reserved.
//

#define MAC	true
#define TTF	true
#define HSOUND	true
#define NI	420 //number of images in the game (number of objects)
#define EDIT	false

#include <fcntl.h>
#include <sys/types.h>
#include <unistd.h>

#if MAC
	const char *RESOURCES = ""; //resource folder in Mac apps
	const char *SOUNDS_DIRECTORY = "Sounds/"; //sounds folder
	const char *LEVELS_DIRECTORY = "Levels/"; //levels folder
	#import <Cocoa/Cocoa.h>
	#import <SDL/SDL.h>
	#if TTF
		#import <SDL_ttf/SDL_ttf.h>
	#endif
#else
	const char *RESOURCES = ""; //resource folder
	const char *SOUNDS_DIRECTORY = "Sounds/"; //sounds folder
	const char *LEVELS_DIRECTORY = "Levels/"; //levels folder
	#include <SDL/SDL.h>
	#if TTF
		#include <SDL/SDL_ttf.h>
	#endif
#endif

#include "read.h"
#include "sound.h"
#include "level.h"
#include "rules.h"
#include "anims.h"

#define FINAL	false
#define CHEATED	false
bool RED_FLASH = true;

#if FINAL && !EDIT
	#define SCREENX 640 //1280 //640 //window dimensions
	#define SCREENY	480 //800 //480

	#define NX	14 //40 //10 //10 //number of visible boxes
	#define NY	10 //24 //8 //8
#else
	#define SCREENX 1280 //640 //window dimensions
	#define SCREENY	800 //480

	#define NX	40 //10 //10 //number of visible boxes
	#define NY	24 //8 //8
#endif

#define CX	32 //box size in pixels
#define CY	25

#import "edit.h"

#define DHEAR	10 //sound distance in boxes
#define DHEAR2	(DHEAR*DHEAR)

#define TANIM	30 //minimum time between two frame animations (in milliseconds)

#if FINAL
	#define TVIEW	20 //time between two steps of a view movement
	#define NVIEW	10 //number of frames of a view movement
	#define NMUS0	5	//number of frames of one of our moves
	Uint32 NMUS	 = NMUS0;	//number of frames of one of our moves
	#define TMUS	30	//time between two frames of one of our moves
#else
	#define TVIEW	5 //time between two steps of a view movement
	#define NVIEW	24 //number of frames of a view movement
	#define NMUS0	5	//number of frames of one of our moves
	Uint32 NMUS	 = NMUS0;	//number of frames of one of our moves
	#define TMUS	20	//time between two frames of one of our moves
#endif

#define tNMUS0	10 //time remaining before the end of the acceleration (in number of boxes)
Uint32 tNMUS = 0; //time remaining before the end of the acceleration

#define NBMAX	1000 //maximum number of balls
#define TBALLS	20	//time between two images for bullets
#define VBALL	3	//displacement every time (number of pixels) - movement?
#define TCONVEYOR_BELT	2 //time between two frames of a conveyor belt

#define INVINCIBLE	300 //invincibility time

Uint8 GetDir (int dx, int dy);
Uint8 GetDir2 (int dx, int dy);
char inline dirx (Uint8 dir);
char inline diry (Uint8 dir);
Sint8 sign (Sint8 n);
void pause (Uint32 t);
Pos GetRailDir (Uint16 c, Uint8 dir); //gives direction to the rail which is depending on the direction of entry 
Pos GetRailDir (Uint16 c); //gives direction to which the rail will go

struct Bullet
{
	Uint16 x,y;	//position
	Sint8	vx,vy; //speed
	//SDL_Surface s;
	bool used; //is ball active
	Uint8 react; //did the bullet react with a box? // 0 = no, 1 = entered the box, 2 = middle of the box
	
	void init ();
};

enum
{
	wwall = 0,
	wplant = 1,
	warrow = 2
};

enum
{
	wup = 1 << 0,
	wdown = 1 << 1,
	wright = 1 << 2,
	wleft = 1 << 3
};

struct Well
{
	Uint16 t[2*2*2*2];
	
	void init (Uint8 which);
};

//win
enum
{
	wasted = 0,
	won = 1,
	kabort = 2,
	ingame = 3
};

//touches
#define NKEYS	8
enum
{
	kup = 0,
	kdown  = 1,
	kright = 2,
	kleft = 3,
	kshoot = 4,
	ktake  = 5,
	kteleport = 6,
	ksuicide = 7
};

struct Game
{
	Sounds sounds; //sound management
	Level t; //current level
	Pos pr; //our position in pixels, relative to the box
	Rules r[NI]; //rules of each object
	Anim a[NA]; //list of events
	Bullet b[NBMAX]; //list of bullets
	SDL_Surface *screen; //screen
	SDL_Surface *im; //game images
	SDL_Surface *board; //image of the complete level
	SDL_Surface *boximg; //image of one box
	Well w[3]; //used by the editor to make adjacent tiles fit together
#if TTF
	TTF_Font *font;
#endif
	SDLKey key[NKEYS]; //list of control keys
	SDL_Rect view; //image area
	SDL_Rect pview; //view position on screen
	bool quit;
	bool quitanim;
	Uint8 win; //did we win?
	bool moveus; //we must move forward?
	bool canmove; //can change direction
	Uint16 invincible; //remaining invincible time
	Uint16 vposx;
	Uint16 vposy;
	
	bool doviewrun;	//used by DoView
	bool updateview; //used by DoView
	bool paused;
	
	void init ();
	void end ();
	void Play (Uint32 level); //start the game in the current level (the argument is only used for cheating)
	//
	void Draw (); //redraw all
	inline void DrawGround (Uint16 x, Uint16 y); //draws the floor of box x,y
	inline void Draw (Uint16 x, Uint16 y); //redraws the box x,y
	inline void DrawUs ();
	void DrawBall (Uint16 x, Uint16 y);
	void MoveUs (Uint8 dir);
	void Update (); //redraw visible area
	void Animate (Uint16 x, Uint16 y, Uint16 n); //starts animation n at (x,y)
	void DoBullet (Uint16 x, Uint16 y); //action of a bullet explosion x,y
	void DoExplosive (Uint16 x, Uint16 y); //start explosive at x,y
	void DoGrenade (Uint16 x, Uint16 y); //start grenade x,y
	void DoDoor (Uint16 x, Uint16 y); //starts an object that reacts when we pass next to it
	void UpdateView (Uint16 x, Uint16 y); //updates the field of view
	void Wasted ();
	void ReactOn (Uint16 x, Uint16 y);
	void OpenDoors ();
	void OpenDoors (Uint16 w);
	void OpenBarrier ();
	void CloseBarrier ();
	void InitBalls ();
	void Bullet(Uint16 x,Uint16  y,int dx,int dy); //shoot bullet from x,y in direction dx,dy
	//void Bullet2(Uint16 x,Uint16  y,int dx,int dy); //shoot bullet from x,y in direction dx,dy from center??
	void StartView (); //puts the field of view where it should be
	Pos GoodView (); //where the field of view should be
	void ActivateRail (Uint16 x, Uint16 y); //activates a rail
	void DeactivateRail (Uint16 x, Uint16 y); //disables a rail
	void Teleport (Uint16 w);
	void DoComputer (Uint16 w);
	void DoPatch (Uint16 x, Uint16 y, Uint16 w);
	void View (); //magnifier
	void MoveViewTo (SDL_Rect nview);
	void MoveViewTo2 (SDL_Rect nview);
	void PutRed (Uint8 red);
	void Main (); //start screen
	int EnterCode ();
	void EditKeys (); //control settings
	void EditSound (); //sound setting
	void EditGraph (); //graphic settings
	void EditParams (); //game settings
	void ChooseLevel (Uint16 *level); //level select
	void SavePrefs (); 
	void LoadPrefs (); 
	void DispInfo (Uint16 level); //display information (level + code)
	void Recenter (); //puts things in place
	void Pause (); //
	void DoPause (); //pause used by the threads
 	//Train
	void initRail (Uint16 x, Uint16 y, Uint16 t, Uint8 dir); //initializes the rail at (x,y) and the following ones
	void initTrain (); //initializes automatic rails
	void DeactivateRails (); //disable all rails
	Uint16 ActivateRail (Uint16 rail); //activates a rail
	Uint16 DeactivateRail (Uint16 rail); //disables a rail
	//
	void Load (Uint16 n); //load level
	void Save (Uint16 n); //save
	void Update_tdir (); //updates tdir (shooting direction)
	//
	void Edit (Uint32 level);
	//void LoadTab (Uint32 n); //load user level
	//void SaveTab (Uint32 n); //save user level
	Uint16 WellThing (Uint16 x, Uint16 y, Uint32 thing, Uint32 wthing);
	void PutWellThing (Uint16 x, Uint16 y, Uint32 thing, Uint32 wthing);
	void PutWell (Uint16 x, Uint16 y, Uint16 what);
};

int max (int a, int b)
{
	if (a < b)
		return b;
	return a;
}

void Well::init (Uint8 which)
{
	switch (which)
	{
		case wwall: //walls
			t[0] = 28;
			t[wleft] = 27;
			t[wright] = 16;
			t[wleft+wright] = 26;
			t[wdown] = 24;
			t[wdown+wleft] = 18;
			t[wdown+wright] = 19;
			t[wdown+wright+wleft] = 72;
			t[wup] = 25;
			t[wup+wleft] = 14;
			t[wup+wright] = 29;
			t[wup+wright+wleft] = 75;
			t[wup+wdown] = 17;
			t[wup+wdown+wleft] = 73;
			t[wup+wdown+wright] = 74;
			t[wup+wdown+wright+wleft] = 15;
			break;
		case wplant: //plants
			t[0] = 68;
			t[wleft] = 76;
			t[wright] = 77;
			t[wleft+wright] = 54;
			t[wdown] = 78;
			t[wdown+wleft] = 57;
			t[wdown+wright] = 56;
			t[wdown+wright+wleft] = 64;
			t[wup] = 79;
			t[wup+wleft] = 59;
			t[wup+wright] = 58;
			t[wup+wright+wleft] = 65;
			t[wup+wdown] = 55;
			t[wup+wdown+wleft] = 66;
			t[wup+wdown+wright] = 67;
			t[wup+wdown+wright+wleft] = 69;
			break;
		case warrow: //arrows
			t[0] = 0;
			t[wleft] = 0;
			t[wright] = 0;
			t[wleft+wright] = 47;
			t[wdown] = 0;
			t[wdown+wleft] = 51;
			t[wdown+wright] = 50;
			t[wdown+wright+wleft] = 0;
			t[wup] = 0;
			t[wup+wleft] = 42;
			t[wup+wright] = 40;
			t[wup+wright+wleft] = 0;
			t[wup+wdown] = 46;
			t[wup+wdown+wleft] = 0;
			t[wup+wdown+wright] = 0;
			t[wup+wdown+wright+wleft] = 0;
			break;
	}
}

void Bullet::init ()
{
	used = false;
	react = false;
}

void Game::SavePrefs ()
{
	FILE *f = fopen("prefs.dat","w+");
	if (f)
	{
		fwrite(&SOUND, sizeof(SOUND), 1, f);
		fwrite(&RED_FLASH, sizeof(RED_FLASH), 1, f);
		Uint32 i;
		for (i=0;i<NKEYS;i++)
		{
			fwrite(&key[i], sizeof(key[i]), 1, f);
		}
		fclose(f);
	}else
	{
		fprintf(stderr, "Error: Unable to open (or create) the preferences file!\n");
	}
}

void Game::LoadPrefs ()
{
	FILE *f = fopen("prefs.dat","r");
	if (f)
	{
		fread(&SOUND, sizeof(SOUND), 1, f);
		fread(&RED_FLASH, sizeof(RED_FLASH), 1, f);
		Uint32 i;
		for (i=0;i<NKEYS;i++)
		{
			fread(&key[i], sizeof(key[i]), 1, f);
		}
		fclose(f);
	}else
	{
		//fprintf(stderr, "Error: Unable to open the preferences file!\n");
	}
}

void Game::Load (Uint16 n)
{
	t.Load(n);
	initTrain();
}

void Game::Save (Uint16 n)
{
	t.Save(n);
}

void FadeIn (SDL_Surface *s, SDL_Surface *screen)
{
	SDL_Event event;
	Uint32 ti,rti;
	SDL_Rect dest;
	rti = SDL_GetTicks();
	int i;
	for (i=0;i<=12;i++)
	{
		ti = SDL_GetTicks();
		if (ti-rti < 50)
			SDL_Delay(50-(ti-rti));
		rti = SDL_GetTicks();
		SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
		dest.x = (screen->w - s->w)/2;
		dest.y = (screen->h - s->h)/2;
		SDL_SetAlpha(s, SDL_SRCALPHA, 20*i+15);
		SDL_BlitSurface(s, NULL, screen, &dest);
		SDL_Flip(screen);
		SDL_PollEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				i = 100;
				SDL_PushEvent(&event);
				break;
			case SDL_KEYDOWN:
				i = 100;
				SDL_PushEvent(&event);
				break;
		}
	}
	SDL_SetAlpha(s, SDL_SRCALPHA, 255);
	dest.x = (screen->w - s->w)/2;
	dest.y = (screen->h - s->h)/2;
	SDL_BlitSurface(s, NULL, screen, &dest);
	SDL_Flip(screen);
}

void FadeOut (SDL_Surface *s, SDL_Surface *screen)
{
	SDL_Event event;
	Uint32 ti,rti;
	rti = SDL_GetTicks();
	int i;
	SDL_Rect dest;
	for (i=0;i<12;i++)
	{
		ti = SDL_GetTicks();
		if (ti-rti < 50)
			SDL_Delay(50-(ti-rti));
		rti = SDL_GetTicks();
		SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
		dest.x = (screen->w - s->w)/2;
		dest.y = (screen->h - s->h)/2;
		SDL_SetAlpha(s, SDL_SRCALPHA, 255 - 20*i);
		SDL_BlitSurface(s, NULL, screen, &dest);
		SDL_Flip(screen);
		SDL_PollEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				i = 100;
				SDL_PushEvent(&event);
				break;
			case SDL_KEYDOWN:
				i = 100;
				SDL_PushEvent(&event);
				break;
		}
	}
	SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
	SDL_Flip(screen);
}

int GetLevel (const char *txt)
{
	char buf[256];
	MakeName("codes.txt", buf, RESOURCES, 256);
	FILE *f = fopen(buf, "r");
	int level = -1;
	Uint16 k;
	int i;
	if (f)
	{
		char buf[256];
		const char *ptr,*ptr2;
		for (;;)
		{
			if (!readline(f, buf, 256))
				break;
			ptr = buf;
			ptr = findnum (ptr, &k);
			//printf("k = %d ",k);
			if (!ptr)
				continue;
			ptr = pass(ptr); //skip spaces
			//printf(ptr);
			//printf("\n");
			ptr2 = txt;
			for (i=0;i<6;i++)
			{
				if (*ptr != *ptr2)
				{
					i = 0;
					break;
				}
				ptr++;
				ptr2++;
			}
			if (i)
			{
				level = k-1;
				break;
			}
		}
		fclose(f);
	}else
	{
		fprintf(stderr, "Unable to open the code file!\n");
	}
	return level;
}

bool GetLevel (int level, char *txt)
{
	char buf[256];
	MakeName("codes.txt", buf, RESOURCES, 256);
	FILE *f = fopen(buf, "r");	
	Uint16 k;
	int i;
	if (f)
	{
		char buf[256];
		const char *ptr;
		char *ptr2;
		for (;;)
		{
			if (!readline(f, buf, 256))
				break;
			ptr = buf;
			ptr = findnum (ptr, &k);
			if (!ptr)
				continue;
			if (k == level)
			{
				ptr = pass(ptr);
				ptr2 = txt;
				for (i=0;i<6;i++)
				{
					*ptr2 = *ptr;
					ptr2++;
					ptr++;
				}
				*ptr2 = 0;
				return true;
			}
		}
		fclose(f);
	}else
	{
		fprintf(stderr, "Unable to open the code file !!!\n");
	}
	return false;
}

int Game::EnterCode ()
{
#if TTF
	SDL_Color white = {255, 255, 255};
	SDL_Rect dest;
	SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
	SDL_Surface *txt = TTF_RenderText_Blended(font, "Enter the code of a level :", white);
	dest.x = (screen->w - txt->w)/2;
	dest.y = 100;
	SDL_BlitSurface(txt, NULL, screen, &dest);
	SDL_Flip(screen);
	SDL_FreeSurface(txt);
	bool cont = true;
	SDL_Event event;
	char buf[255]; //entered text
	Uint16 pos = 0; //cursor position (number of letters)
	bool update = false;
	bool wrong = false;
	SDL_EnableUNICODE(1);
	Uint16 c;
	int level = -1;
	do
	{
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_BACKSPACE:
						if (pos)
						{
							sounds.Play(16); //bomb explosion
							pos--;
							buf[pos] = 0;
							update = true;
						}
						break;
					case SDLK_RETURN:
						level = GetLevel(buf);
						if (level != -1)
						{
							sounds.Play(4);
							cont = false;
						}else
						{
							wrong = true;
						}
						break;
				}
				c = toupper(event.key.keysym.unicode); //event.key.keysym.sym; //event.key.keysym.unicode);
				if (c >= 'A' && c <= 'Z') //SDLK_a && c <= SDLK_z)
				{
					if (pos < 254)
					{
						sounds.Play(13);
						buf[pos] = c;
						pos++;
						buf[pos] = 0;
						update = true;
					}
				}
				break;
		}
		if (update || wrong)
		{
			dest.x = 200;
			dest.y = 200;
			dest.w = 400;
			dest.h = 50;
			SDL_FillRect(screen, &dest, SDL_MapRGB(screen->format, 0, 0, 0));
			txt = TTF_RenderText_Blended(font, buf, white);
			dest.x = 200;
			dest.y = 200;
			SDL_BlitSurface(txt, NULL, screen, &dest);
			SDL_Flip(screen);
			SDL_FreeSurface(txt);
			if (wrong)
			{
				SDL_FillRect(screen, &dest, SDL_MapRGB(screen->format, 0, 0, 0));
				txt = TTF_RenderText_Blended(font, "Bad code!", white);
				dest.x = 200;
				dest.y = 300;
				SDL_BlitSurface(txt, NULL, screen, &dest);
				SDL_Flip(screen);
				SDL_FreeSurface(txt);
			}else
			{
				dest.x = 200;
				dest.y = 300;
				dest.w = 400;
				dest.h = 50;
				SDL_FillRect(screen, &dest, SDL_MapRGB(screen->format, 0, 0, 0));
			}
			update = false;
			wrong = false;
		}
	}while(cont);
	SDL_EnableUNICODE(0);
	return level;
#endif
}
/*
void DrawTxt (const char *texte, Uint16 x, Uint16 y, SDL_Surface *screen)
{
	SDL_Color white = {255, 255, 255};
	SDL_Surface *txt = TTF_RenderText_Blended(font, texte, white);
	dest.x = x;
	dest.y = y;
	SDL_BlitSurface(txt, NULL, screen, &dest);
}
*/

void Game::EditSound ()
{
	SDL_Rect dest;
	char buf[255];
	SDL_Color white = {255, 255, 255};
	SDL_Surface *txt;
	Uint32 c = 0;
	bool cont = true;
	bool update = true;
	SDL_Event event;
	Uint32 keyindex;
	do
	{
		if (update)
		{
			update = false;
			SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
#if TTF
			if (SOUND)
				txt = TTF_RenderText_Blended(font, "F1 Mute sound", white);
			else
				txt = TTF_RenderText_Blended(font, "F1 Unmute sound", white);
			dest.x = 20;
			dest.y = 20;
			SDL_BlitSurface(txt, NULL, screen, &dest);
			SDL_FreeSurface(txt);
			txt = TTF_RenderText_Blended(font, "ESC Back", white);
			dest.x = 20;
			dest.y = 50;
			SDL_BlitSurface(txt, NULL, screen, &dest);
			SDL_FreeSurface(txt);
#endif
			SDL_Flip (screen);
		}
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_F1:
						SOUND = !SOUND;
						sounds.Play(4);
						cont = false;
						break;
				}
				break;
		}
	}while(cont);
}

void Game::EditGraph ()
{
	SDL_Rect dest;
	char buf[255];
	SDL_Color white = {255, 255, 255};
	SDL_Surface *txt;
	Uint32 c = 0;
	bool cont = true;
	bool update = true;
	SDL_Event event;
	Uint32 keyindex;
	do
	{
		if (update)
		{
			update = false;
			SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
#if TTF
			if (RED_FLASH)
				txt = TTF_RenderText_Blended(font, "F1 Disable red flashing", white);
			else
				txt = TTF_RenderText_Blended(font, "F1 Enable red flashing", white);
			dest.x = 20;
			dest.y = 20;
			SDL_BlitSurface(txt, NULL, screen, &dest);
			SDL_FreeSurface(txt);
			txt = TTF_RenderText_Blended(font, "ESC Back", white);
			dest.x = 20;
			dest.y = 50;
			SDL_BlitSurface(txt, NULL, screen, &dest);
			SDL_FreeSurface(txt);
#endif
			SDL_Flip (screen);
		}
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_F1:
						RED_FLASH = !RED_FLASH;
						sounds.Play(4);
						cont = false;
						break;
				}
				break;
		}
	}while(cont);
}

void Game::EditKeys () //allows the user to change the control keys
{
	SDL_Rect dest;
	char buf[255];
	SDL_Color white = {255, 255, 255};
	SDL_Surface *txt;
	Uint32 c = 0;
	bool cont = true;
	bool update = true;
	SDL_Event event;
	Uint32 keyindex;
	do
	{
		if (update)
		{
			update = false;
			SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
#if TTF
			switch (c)
			{
				case 0:
					txt = TTF_RenderText_Blended(font, "Key to move left ?", white);
					break;
				case 1:
					txt = TTF_RenderText_Blended(font, "Key to move right?", white);
					break;
				case 2:
					txt = TTF_RenderText_Blended(font, "Key to move up?", white);
					break;
				case 3:
					txt = TTF_RenderText_Blended(font, "Key to move down?", white);
					break;
				case 4:
					txt = TTF_RenderText_Blended(font, "Key to take/drop ?", white);
					break;
				case 5:
					txt = TTF_RenderText_Blended(font, "Key to shoot?", white);
					break;
				case 6:
					txt = TTF_RenderText_Blended(font, "Key to teleport?", white);
					break;
				case 7:
					txt = TTF_RenderText_Blended(font, "Key to give up?", white);
					break;
			}
			dest.x = 20;
			dest.y = 20;
			SDL_BlitSurface(txt, NULL, screen, &dest);
			dest.x += txt->w;
			SDL_FreeSurface(txt);
#endif
			SDL_Flip (screen);
		}
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (c)
				{
					case 0:
						keyindex = kleft;
						break;
					case 1:
						keyindex = kright;
						break;
					case 2:
						keyindex = kup;
						break;
					case 3:
						keyindex = kdown;
						break;
					case 4:
						keyindex = ktake;
						break;
					case 5:
						keyindex = kshoot;
						break;
					case 6:
						keyindex = kteleport;
						break;
					case 7:
						keyindex = ksuicide;
						break;
				}
				key[keyindex] = event.key.keysym.sym;
				sounds.Play(4); //lock
				c++;
				if (c > 7)
					return;
					update = true;
		}
	}while(cont);
}

void Game::EditParams ()
{
	SDL_Rect dest;
	char buf[255];
	SDL_Color white = {255, 255, 255};
	SDL_Surface *txt;
	Uint32 c = 0;
	bool cont = true;
	bool update = true;
	SDL_Event event;
	Uint32 keyindex;
	do
	{
		if (update)
		{
			update = false;
			SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
#if TTF
			dest.x = 20;
			dest.y = 20;
			txt = TTF_RenderText_Blended(font, "Parameters", white);
			SDL_BlitSurface(txt, NULL, screen, &dest);
			dest.x += txt->w;
			SDL_FreeSurface(txt);
			dest.x = 20;
			dest.y = 90;
			txt = TTF_RenderText_Blended(font, "F1 Changes Keys", white);
			SDL_BlitSurface(txt, NULL, screen, &dest);
			dest.x += txt->w;
			SDL_FreeSurface(txt);
			dest.x = 20;
			dest.y = 140;
			txt = TTF_RenderText_Blended(font, "F2 Adjust Sound", white);
			SDL_BlitSurface(txt, NULL, screen, &dest);
			dest.x += txt->w;
			SDL_FreeSurface(txt);
			dest.x = 20;
			dest.y = 190;
			txt = TTF_RenderText_Blended(font, "F3 Graphics", white);
			SDL_BlitSurface(txt, NULL, screen, &dest);
			dest.x += txt->w;
			SDL_FreeSurface(txt);
			dest.x = 20;
			dest.y = 240;
			txt = TTF_RenderText_Blended(font, "ESC Back", white);
			SDL_BlitSurface(txt, NULL, screen, &dest);
			dest.x += txt->w;
			SDL_FreeSurface(txt);
#endif
			SDL_Flip (screen);
			SavePrefs ();
		}
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_F1:
						EditKeys();
						update = true;
						break;
					case SDLK_F2:
						EditSound();
						update = true;
						break;
					case SDLK_F3:
						EditGraph();
						update = true;
						break;
				}
		}
	}while(cont);
}

void Game::DispInfo (Uint16 level)
{
	level++;
	
	SDL_FillRect(screen, NULL, SDL_MapRGB(screen->format, 0, 0, 0));
	SDL_Rect dest;
	char buf[255];
	SDL_Color orange = {255, 200, 100};
	SDL_Surface *txt;
#if TTF
	txt = TTF_RenderText_Blended(font, "Level ", orange);
	dest.x = 20;
	dest.y = 20;
	SDL_BlitSurface(txt, NULL, screen, &dest);
	dest.x += txt->w;
	SDL_FreeSurface(txt);
	buf[2] = '0' + (level%10);
	buf[1] = '0' + ((level/10)%10);
	buf[0] = '0' + (level/100);
	buf[3] = 0;
	if (buf[0] == '0' && buf[1] == '0')
		buf[1] = ' ';
	if (buf[0] == '0')
		buf[0] = ' ';
	txt = TTF_RenderText_Blended(font, buf, orange);
	SDL_BlitSurface(txt, NULL, screen, &dest);
	SDL_FreeSurface(txt);
	
	if (GetLevel(level, buf))
	{
		txt = TTF_RenderText_Blended(font, "Code ", orange);
		dest.x = 20;
		dest.y = 40;
		SDL_BlitSurface(txt, NULL, screen, &dest);
		dest.x += txt->w;
		SDL_FreeSurface(txt);
		txt = TTF_RenderText_Blended(font, buf, orange);
		SDL_BlitSurface(txt, NULL, screen, &dest);
		SDL_FreeSurface(txt);
	}
#endif
}

void Game::Main ()
{
	//show the Kalisto image
	char buf[256];
	MakeName("Kalisto.bmp", buf, RESOURCES, 256);
	SDL_Surface *s = SDL_LoadBMP (buf);
	//SDL_Surface *sf = SDL_CreateRGBSurface(SDL_HWSURFACE, s->w, s->h, 32, 0, 0, 0, 0); //background
	if (!s)
		fprintf(stderr, "Kalisto image missing\n");
	int i;
	Uint32 ti,rti;
	SDL_Event event;
	SDL_Rect src, dest;
	FadeIn(s, screen);
	bool cont = true;
	rti = SDL_GetTicks();
	do
	{
		ti = SDL_GetTicks();
		if (ti-rti > 2000)
			break;
		SDL_Delay(10);
		SDL_PollEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				SDL_PushEvent(&event);
				cont = false;
				break;
			case SDL_KEYDOWN:
				SDL_PushEvent(&event);
				cont = false;
				break;
		}
	}while(cont);
	FadeOut(s, screen);
	SDL_FreeSurface(s);
	
	//show the start screen
	MakeName("S.C.OUT.bmp",buf, RESOURCES, 256);
	s = SDL_LoadBMP (buf);
	SDL_Color orange = {255, 200, 100};
#if TTF
	SDL_Surface *txt = TTF_RenderText_Blended(font, "F1 Play", orange);
	dest.x = 20;
	dest.y = 300;
	SDL_BlitSurface(txt, NULL, s, &dest);
	SDL_FreeSurface(txt);
	txt = TTF_RenderText_Blended(font, "F2 Enter Code", orange);
	dest.x = 20;
	dest.y = 320;
	SDL_BlitSurface(txt, NULL, s, &dest);
	SDL_FreeSurface(txt);
	txt = TTF_RenderText_Blended(font, "F3 Settings", orange);
	dest.x = 20;
	dest.y = 340;
	SDL_BlitSurface(txt, NULL, s, &dest);
	SDL_FreeSurface(txt);
	txt = TTF_RenderText_Blended(font, "ESC Quit", orange);
	dest.x = 350;
	dest.y = 340;
	SDL_BlitSurface(txt, NULL, s, &dest);
	SDL_FreeSurface(txt);
#endif
	FadeIn(s, screen);
	cont = true;
	Uint16 level = 0;
	int lvl;
	do
	{
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_F1:
						FadeOut(s, screen);
						for (;;)
						{
							DispInfo(level);
							Load(level);
							Play(level);
							if (win == won)
								level++;
							if (win == kabort)
								break;
						}
						FadeIn(s, screen);
						break;
					case SDLK_F2:
						FadeOut(s, screen);
						lvl = EnterCode();
						if (lvl != -1)
							level = lvl;
						FadeIn(s, screen);
						break;
					case SDLK_F3:
						FadeOut(s, screen);
						EditParams ();
						FadeIn(s, screen);
						break;
#if !FINAL || CHEATED
					case SDLK_F4:
						FadeOut(s, screen);
						ChooseLevel (&level);
						printf("level = %d\n",level);
						FadeIn(s, screen);
						break;
#endif
#if EDIT || !FINAL
					case SDLK_F5:
						FadeOut(s, screen);
						Load(level);
						Edit(level);
						FadeIn(s, screen);
						break;
#endif
				}
		}
	}while(cont);
	SDL_FreeSurface(s);
}

/*
void Game::SaveTab (Uint32 n) //saves a level created by the user
{
	char txt[40];
	sprintf(txt,"level%d",n);
	t.Save(txt);
}
*/

Uint16 Game::WellThing (Uint16 x, Uint16 y, Uint32 thing, Uint32 wthing)
{
	Uint32 m = 0;
	if ((Uint16)(x-1) < DIMX)
		if ((r[t.t[x-1][y]].is(thing) && wthing != warrow) || (r[t.ground[x-1][y]].is(thing) && wthing == warrow))
			m+=wleft;
	if ((Uint16)(x+1) < DIMX)
		if ((r[t.t[x+1][y]].is(thing) && wthing != warrow) || (r[t.ground[x+1][y]].is(thing) && wthing == warrow))
			m+=wright;
	if ((Uint16)(y-1) < DIMY)
		if ((r[t.t[x][y-1]].is(thing) && wthing != warrow) || (r[t.ground[x][y-1]].is(thing) && wthing == warrow))
			m+=wup;
	if ((Uint16)(y+1) < DIMY)
		if ((r[t.t[x][y+1]].is(thing) && wthing != warrow) || (r[t.ground[x][y+1]].is(thing) && wthing == warrow))
			m+=wdown;
	//printf("m = %d\n",m);
	return w[wthing].t[m];
}

void Game::PutWellThing (Uint16 x, Uint16 y, Uint32 thing, Uint32 wthing)
{
	Uint16 c;
	int i,j;
	for (i=-1;i<=1;i++)
	{
		for (j=-1;j<=1;j++)
		{
			if (!i || !j)
			{
				if ((Uint16)(x+i) < DIMX && (Uint16)(y+j) < DIMY)
				{
					if ((r[t.t[x+i][y+j]].is(thing) && wthing != warrow) || (r[t.ground[x+i][y+j]].is(thing) && wthing == warrow))
					{
						//printf("(%d,%d) : %d\n",i,j,t.t[x+i][y+j]);
						c = WellThing(x+i, y+j, thing, wthing);
						if (c)
						{
							//printf("c = %d\n",c);
							if (wthing == warrow)
								t.ground[x+i][y+j] = c;
							else
								t.t[x+i][y+j] = c;
							Draw(x+i, y+j);
						}
					}
				}
			}
		}
	}
}

void Game::PutWell (Uint16 x, Uint16 y, Uint16 what)
{
	if (r[what].is(wellable) || t.t[x][y] == 0)
	{
		if (r[what].is(real_wall))
		{
			//printf("put well thing...\n");
			PutWellThing (x, y, real_wall, wwall);
		}
		if (r[what].is(real_plant))
		{
			PutWellThing (x, y, real_plant, wplant);
		}
		if (r[what].is(ground_arrow))
		{
			PutWellThing (x, y, ground_arrow, warrow);
		}
		if (r[what].is(rail))
		{
			//////////////
		}
	}
}

void Game::Edit (Uint32 level)
{
	struct Edit e;
	printf("loading...\n");
	e.Load();
	pview.y = 0;
	printf("drawing...\n");
	Draw();
	SDL_BlitSurface(board, &view, screen, &pview);
	e.Draw(im, screen);
	SDL_Flip(screen);
	Update(); //
	SDL_Event event;
	bool cont = true;
	printf("loop...\n");
	bool fill = false;
	Uint16 x,y;
	Uint16 rt;
	do
	{
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_d:
						Draw();
						Update();
						break;
					case SDLK_e:
						Save(level);
						break;
					case SDLK_c:
						Load(level);
						break;
				}
			case SDL_MOUSEBUTTONDOWN:
				x = event.motion.x/CX;
				y = event.motion.y/CY;
				if (x < DIMX && y < DIMY)
				{
					//fill = true;
				}else
				{
					e.Select(x,y-DIMY);
				}
			case SDL_MOUSEMOTION:
				fill = (SDL_GetMouseState(NULL, NULL)&SDL_BUTTON(1)); //(event.motion.state == SDL_PRESSED);
				x = event.motion.x/CX;
				y = event.motion.y/CY;
				if (fill)
				{
					if (x < DIMX && y < DIMY)
					{
						if (r[e.sel].is(ground))
						{
							rt = t.ground[x][y];
							t.ground[x][y] = e.sel;
						}else
						{
							rt = t.t[x][y];
							t.t[x][y] = e.sel;
						}
						if (!e.sel)
							PutWell(x,y,rt);
						else
							PutWell(x,y,e.sel);
						Draw(x,y);
						//SDL_BlitSurface(board, &view, screen, &pview);
						//SDL_Flip(screen);
						Update();
					}
				}
				break;
		}
	}while(cont);
}

void Game::DoPatch (Uint16 x, Uint16 y, Uint16 w)
{
	int dx,dy;
	for (dx=-1;dx<=1;dx++)
	{
		for (dy=-1;dy<=1;dy++)
		{
			if ((Uint16)(x+dx) < DIMX && (Uint16)(y+dy) < DIMY)
			{
				if (!t.t[x+dx][y+dy] || (!dx && !dy))
				{
					if (w == 164)//green patch
						Animate(x+dx, y+dy, 14);
					else
						Animate(x+dx, y+dy, 15);
				}
			}
		}
	}
}

void Game::DoComputer (Uint16 w)
{
	Uint16 x,y;
	Uint16 na = 0;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == 168 || t.t[x][y] == 218) //open or closed computer door
			{
				na = 0;
				switch (w)
				{
					case 197:
						if (t.t[x][y] == 218) //open
							na = 13; //closed
						break;
					case 207:
						if (t.t[x][y] == 168) //closed
							na = 12; //open
						else
							na = 13; //closed
						break;
					case 217:
						if (t.t[x][y] == 168) //closed
							na = 12; //open
						break;
				}
				if (na)
					Animate(x, y, na);
			}
		}
	}
}

Uint16 Game::ActivateRail (Uint16 wrail) //activates a rail
{
	if (r[wrail].is(rail))
	{
		if (wrail%10 < 2)
		{
			if (((int)(wrail/10))%2 == 0)
				wrail += 10;
		}
	}
	return wrail;
}

Uint16 Game::DeactivateRail (Uint16 wrail) //deactivates a rail
{
	if (r[wrail].is(rail))
	{
		if (wrail%10 < 2)
		{
			if (((int)(wrail/10))%2 == 1)
				wrail -= 10;
		}
	}
	return wrail;
}

Pos GetRailDir (Uint16 c, Uint8 dir) //gives the direction the rail leads to, depending on the direction of entry
{
	Pos p;
	p.x = p.y = 0;
	if ((c == 170 || c == 210 || c == 160 || c == 200)) // && dir == left) //left
		p.x = -1;
	if ((c == 190 || c == 230 || c == 180 || c == 220)) // && dir == right) //right
		p.x = 1;
	if ((c == 171 || c == 211 || c == 161 || c == 201)) // && dir == up) //up
		p.y = -1;
	if ((c == 191 || c == 231 || c == 181 || c == 221)) // && dir == down) //down
		p.y = 1;
	if (c == 162 && dir == right) //right rail -> down
		p.y = 1;
	if (c == 172 && dir == left) //left rail -> down
		p.y = 1;
	if (c == 182 && dir == down) //bottom rail -> right
		p.x = 1;
	if (c == 192 && dir == right) //right rail -> up
		p.y = -1;
	if (c == 202 && dir == up) //top rail  -> left
		p.x = -1;
	if (c == 212 && dir == up) //top rail -> right
		p.x = 1;
	if (c == 222 && dir == left) //left rail -> up
		p.y = -1;
	if (c == 232 && dir == down) //bottom rail -> left
		p.x = -1;
	return p;
}

Pos GetRailDir2 (Uint16 c, Uint8 dir) //gives the direction the rail leads to, depending on the direction of entry
{
	Pos p;
	p.x = p.y = 0;
	if ((c == 170 || c == 210 || c == 160 || c == 200) && dir == left) //left
		p.x = -1;
	if ((c == 190 || c == 230 || c == 180 || c == 220) && dir == right) //right
		p.x = 1;
	if ((c == 171 || c == 211 || c == 161 || c == 201) && dir == up) //up
		p.y = -1;
	if ((c == 191 || c == 231 || c == 181 || c == 221) && dir == down) //down
		p.y = 1;
	if (c == 162 && dir == right) //right rail -> down
		p.y = 1;
	if (c == 172 && dir == left) //left rail -> down
		p.y = 1;
	if (c == 182 && dir == down) //bottom rail -> right
		p.x = 1;
	if (c == 192 && dir == right) //right rail  -> up
		p.y = -1;
	if (c == 202 && dir == up) //top rail -> left
		p.x = -1;
	if (c == 212 && dir == up) //top rail -> right
		p.x = 1;
	if (c == 222 && dir == left) //left rail -> up
		p.y = -1;
	if (c == 232 && dir == down) //bottom rail -> left
		p.x = -1;
	return p;
}

Pos GetRailDir (Uint16 c) //gives the direction the rail leads to
{
	Pos p;
	p.x = p.y = 0;
	if (c == 170 || c == 210 || c == 160 || c == 200) //left
		p.x = -1;
	if (c == 190 || c == 230 || c == 180 || c == 220) //right
		p.x = 1;
	if (c == 171 || c == 211 || c == 161 || c == 201) //up
		p.y = -1;
	if (c == 191 || c == 231 || c == 181 || c == 221) //down
		p.y = 1;
	if (c == 162) //right rail -> down
		p.y = 1;
	if (c == 172) //left rail -> down
		p.y = 1;
	if (c == 182) //bottom rail -> right
		p.x = 1;
	if (c == 192) //right rail -> up
		p.y = -1;
	if (c == 202) //top rail -> left
		p.x = -1;
	if (c == 212) //top rail -> right
		p.x = 1;
	if (c == 222) //left rail -> up
		p.y = -1;
	if (c == 232) //bottom rail -> left
		p.x = -1;
	return p;
}

void Game::ActivateRail (Uint16 x, Uint16 y) //activate rail
{
	t.t[x][y] = ActivateRail(t.t[x][y]);
}

void Game::DeactivateRail (Uint16 x, Uint16 y) //deactivate rail
{
	t.t[x][y] = DeactivateRail(t.t[x][y]);
}

void Game::OpenDoors (Uint16 c) //opens all doors of color c (after shooting at a colored circle)
{
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == 80 + (c%10)) //door color
			{
				//printf("opening a door at (%d,%d)...\n",x,y);
				Animate(x, y, c%10);
			}
		}
	}
}

void Game::OpenBarrier ()
{
	//printf("opening barrier...\n");
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == 139 || t.a[x][y].n == 10) //close door
			{
				//printf("opening a door at (%d,%d)...\n",x,y);
				Animate(x, y, 9);
			}
		}
	}
}

void Game::CloseBarrier ()
{
	//printf("closing barrier...\n");
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == 89 || t.a[x][y].n == 9) //open door
			{
				//printf("opening a door at (%d,%d)...\n",x,y);
				Animate(x, y, 10);
			}
		}
	}
}

void Game::OpenDoors () //opens every door (after having put the key in the lock)
{
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == 80) //door
			{
				//printf("opening a door at (%d,%d)...\n",x,y);
				Animate(x, y, 31);
			}
		}
	}
}

void Game::DoDoor (Uint16 x, Uint16 y) //open one-way door
{
	if (!t.a[x][y].n)
		Animate(x, y, t.t[x][y]%10);
}

Pos Game::GoodView () //where the field of view should be
{
	Pos p;
	p.x = t.posx - NX/2;
	p.y = t.posy - NY/2;
	if (p.x < 0)
		p.x = 0;
	if (p.y < 0)
		p.y = 0;
	if (p.x+NX > DIMX)
		p.x = DIMX-NX;
	if (p.y+NY > DIMY)
		p.y = DIMY-NY;
	p.x = p.x*CX;
	p.y = p.y*CY;
	return p;
};

void Game::StartView ()
{
	Pos p;
	p = GoodView();
	view.x = p.x;
	view.y = p.y;
}

Sint8 sign (Sint8 n)
{
	if (n > 0)
		return 1;
	if (n < 0)
		return -1;
	return 0;
}

void Game::Bullet(Uint16 x,Uint16  y,int dx,int dy) //throws a ball from (x, y) in the direction (dx, dy)
{
	int i;
	if ((Uint16)(x+dx) < DIMX)
	{
		if (r[t.t[x+dx][y]].is(wall))
		{
			if ((Uint16)(y+dy) < DIMY)
			{
				if (r[t.t[x][y+dy]].is(wall))
				{
					return;
				}
			}
		}
	}
	for (i=0;i<NBMAX;i++)
	{
		if (!b[i].used)
		{
			b[i].x = (2*x+1)*CX/2;
			b[i].y = (2*y+1)*CY/2;
			b[i].x += dx*(CX/2);
			b[i].y += dy*(CY/2);
			b[i].vx = dx*VBALL;
			b[i].vy = dy*VBALL;
			b[i].used = true;
			b[i].react = 0;
			break;
		}
	}
}
/*
void Game::Bullet2(Uint16 x,Uint16  y,int dx,int dy) //throws a ball from (x, y) in the direction (dx, dy)
{
	int i;
	for (i=0;i<NBMAX;i++)
	{
		if (!b[i].used)
		{
			b[i].x = (2*x+1)*CX/2;
			b[i].y = (2*y+1)*CY/2;
			b[i].vx = dx*VBALL;
			b[i].vy = dy*VBALL;
			b[i].used = true;
			b[i].react = 0;
			break;
		}
	}
}
*/
void Game::MoveViewTo (SDL_Rect nview)
{
	Uint32 ti,rti;
	rti = SDL_GetTicks();
	SDL_Rect rview = view;
	Uint16 i;
	for (i = 0;i < 20; i++)
	{
		SDL_PumpEvents();
		ti = SDL_GetTicks();
		if (ti-rti < 50)
			SDL_Delay(50 - (ti-rti));
		rti = SDL_GetTicks();
		
		view.x = (rview.x*(20-i) + nview.x*i)/20;
		view.y = (rview.y*(20-i) + nview.y*i)/20;
		//Update();
	}
}

void Game::MoveViewTo2 (SDL_Rect nview)
{
	Uint32 ti,rti;
	rti = SDL_GetTicks();
	SDL_Rect rview = view;
	Uint16 i;
	for (i = 0;i <= NVIEW; i++)
	{
		ti = SDL_GetTicks();
		if (ti-rti < TVIEW)
			SDL_Delay(TVIEW - (ti-rti));
		rti = SDL_GetTicks();
		
		view.x = (rview.x*(NVIEW-i) + nview.x*i)/NVIEW;
		view.y = (rview.y*(NVIEW-i) + nview.y*i)/NVIEW;
		//Update();
	}
}

void Game::Teleport (Uint16 w)
{
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == w+10) //arrive !
			{
				Draw(t.posx, t.posy);
				t.posx = x;
				t.posy = y;
				sounds.Play(10);
				SDL_Rect nview;
				Pos p = GoodView();
				nview.x = p.x;
				nview.y = p.y;
				nview.w = view.w;
				nview.h = view.h;
				MoveViewTo(nview);
				sounds.Play (12);
				DrawUs();
				return;
			}
		}
	}
}

void Game::View () //zoom
{
	SDL_Event event;
	SDL_Rect rview = view; //retains the old position
	bool cont = true;
	Uint32 time = 5000; //time view
	Uint32 ti,rti;
	rti = SDL_GetTicks();
	Uint8 *keystate;
	do
	{
		ti = SDL_GetTicks();
		if (ti-rti < 40)
			SDL_Delay(40 - (ti-rti));
		rti = SDL_GetTicks();
		time -= 40;
		if (time < 40)
			break;
		SDL_PollEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				win = kabort;
				quit = true;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						win = kabort;
						quit = true;
						break;
				}
				break;
		}
		keystate = SDL_GetKeyState (NULL);
		if (keystate[SDLK_UP])
		{
			view.y -= 10;
		}
		if (keystate[SDLK_DOWN])
		{
			view.y += 10;
		}
		if (keystate[SDLK_RIGHT])
		{
			view.x += 10;
		}
		if (keystate[SDLK_LEFT])
		{
			view.x -= 10;
		}
		if (view.x < 0)
			view.x = 0;
		if (view.x + view.w >= DIMX*CX)
			view.x = DIMX*CX-view.w;
		if (view.y < 0)
			view.y = 0;
		if (view.y + view.h >= DIMY*CY)
			view.y = DIMY*CY-view.h;
		//Update();
	}while(cont && !quit);
	if (!quit)
	{
		//returns to the initial position
		MoveViewTo(rview);
	}
	t.t[t.posx][t.posy] = 0;
}

void Game::ReactOn (Uint16 x, Uint16 y)
{
	Uint16 c = t.t[x][y];
	//Uint16 c2 = t.t[x][y];
	int dx, dy;
	if (c == 285 || c == 295 || c == 305 || c == 315) //shooter
	{
		dx = dy = 0;
		if (c == 285)
			dx = -1;
		if (c == 295)
			dx = 1;
		if (c == 305)
			dy = 1;
		if (c == 315)
			dy = -1;
		sounds.Play(5);
		//printf("Shooting a bullet from (%d,%d) towards (%d,%d)..\n",x,y,dx,dy);
		Bullet(x, y, dx, dy); //throws a ball from (x, y) in the direction (dx, dy)
	}
	if (c == 310) //view
	{
		sounds.Play(15);
		View ();
	}
	if (c == 307) //accelerator
	{
		tNMUS = tNMUS0;
		NMUS = NMUS0/2;
		sounds.Play(1);
		t.t[x][y] = 0;
	}
	if (c == 281) //invincibility
	{
		sounds.Play(1);
		t.t[x][y] = 0;
		invincible = INVINCIBLE;
	}
	//printf("react_on !\n");
	if (c == 283 || c == 297) //respawn point
	{
		t.sx = x;
		t.sy = y;
		t.so = t.object;
		//printf("saving the position...\n");
	}
}

void Game::Wasted ()
{
	if (quit)
		return;
	if (t.t[t.posx][t.posy] != 317 && !invincible) //invincibility box, or we are invincible
	{
		Animate(t.posx, t.posy, 27);
		if (win != won)
			win = wasted;
		quit = true;
	}else
	{
		if (invincible)
		{
			printf("invincible (%d)\n",invincible);
			DoBullet(t.posx, t.posy);
		}
	}
}

void Game::DoBullet (Uint16 x, Uint16 y) //bullet or explosion (x,y)
{
	int dx,dy;
	Uint16 c = t.t[x][y];
	if (x == t.posx && y == t.posy)
	{
		if (!invincible)
			Wasted ();
	}
	if (r[c].is(rail_turning)) //rail turning
	{
		switch (t.t[x][y])
		{
			case 162: //bottom left
				c = 192; //top left
				break;
			case 192: //top left
				c = 162; //bottom left
				break;
			case 172: //bottom right
				c = 222; //top right
				break;
			case 222: //top right
				c = 172; //bottom right
				break;
			case 182: //top right
				c = 232; //top left
				break;
			case 232: //top left
				c = 182; //bottom right
				break;
			case 202: //bottom left
				c = 212; //bottom right
				break;
			case 212: //bottom right
				c = 202; //bottom left
				break;
		}
		t.t[x][y] = c;
		Draw(x, y);
		//printf("Changing rail...\n");
	}
	if (c == 233) //thing that is destroyed after 10 bullets
	{
		t.a[x][y].t++;
		if (t.a[x][y].t >= 10)
		{
			sounds.Play(16);
			Animate(x, y, 28);
		}
	}
	if (r[c].is(onda_mania_moving))
	{
		dx = dy = 0;
		if (t.a[x][y].n == 19 || t.a[x][y].n == 23) //onda-mania left box
			dx = 1;
		if (t.a[x][y].n == 20 || t.a[x][y].n == 24) //onda-mania right box
			dx = -1;
		if (t.a[x][y].n == 21 || t.a[x][y].n == 25) //onda-mania top box
			dy = 1;
		if (t.a[x][y].n == 22 || t.a[x][y].n == 26) //onda-mania bottom box
			dy = -1;
		if (dx || dy)
		{
			//printf("dx = %d, dy = %d\n",dx,dy);
			if ((Uint16)(x+dx) < DIMX && (Uint16)(y+dy) < DIMY)
			{
				Animate(x+dx, y+dy, 28); //explosion
			}
		}
		sounds.Play(6);
		Animate(x, y, 28); //explosion
	}
	if (c == 285 || c == 295 || c == 305 || c == 315) //shooter
	{
		dx = dy = 0;
		if (c == 285)
			dx = -1;
		if (c == 295)
			dx = 1;
		if (c == 305)
			dy = 1;
		if (c == 315)
			dy = -1;
		sounds.Play(5);
		//printf("Shooting a bullet from (%d,%d) towards (%d,%d)..\n",x,y,dx,dy);
		Bullet(x, y, dx, dy); //throws a ball from (x, y) in the direction (dx, dy)
	}
	if (c == 287) //shooter down special
	{
		sounds.Play(5);
		Bullet(x, y, 0, 1);
	}
	if (r[t.t[x][y]].is(explosive)) //explosive
	{
		DoExplosive(x, y);
	}
	if (t.t[x][y] == 164) //green patch
	{
		DoPatch(x,y,164);
	}
	if (t.t[x][y] == 165) //gray patch
	{
		DoPatch(x,y,165);
	}
	if (t.t[x][y] == 169) //computer
	{
		dx = dy = 0;
		for (dy=-1;dy<=1;dy++)
		{
			for (dx=-1;dx<=1;dx++)
			{
				if ((Uint16)(x+dx) < DIMX && (Uint16)(y+dy) < DIMY)
				{
					if (t.ground[x+dx][y+dy] == 187)
					{
						if (r[t.t[x+dx][y+dy]].is(computer_thing))
						{
							DoComputer (t.t[x+dx][y+dy]);
							return;
						}
					}
				}
			}
		}
	}
	if (r[t.t[x][y]].is(arrow)) //arrows
	{
		Animate(x, y, 28); //explosion
		switch (t.t[x][y])
		{
			case 286: //top
				t.dir = up;
				//MoveUs(up);
				break;
			case 296: //right
				t.dir = right;
				//MoveUs(right);
				break;
			case 306: //bottom
				t.dir = down;
				//MoveUs(down);
				break;
			case 316: //left
				t.dir = left;
				//MoveUs(left);
				break;
		}
		canmove = true;
		moveus = true;
	}
	if (t.t[x][y] == 166) //virus
	{
		for (dx=-1;dx<=1;dx++)
		{
			for (dy=-1;dy<=1;dy++)
			{
				if (!dx || !dy)
				{
					if ((Uint16)(x+dx) < DIMX && (Uint16)(y+dy) < DIMY)
					{
						if (!t.t[x+dx][y+dy] || (t.t[x][y] == 166 && !dx && !dy))
							Animate(x+dx, y+dy, 16);
					}
				}
			}
		}
	}
	if (t.t[x][y] == 144) //grenade
	{
		DoGrenade(x, y);
	}
	if (r[t.t[x][y]].is(plant)) //plant
	{
		sounds.Play(16);
		Animate(x, y, 28); //explosion
	}
	if (r[t.t[x][y]].is(colored_circle)) //colored circle
	{
		sounds.Play(16);
		Animate(x, y, 28); //explosion
		OpenDoors(t.t[x][y]);
	}
	if (t.t[x][y] == 234) //multi-missiles
	{
		sounds.Play(16);
		Animate(x, y, 28); //explosion
		if (x+1 < DIMX)
		{
			if (t.t[x+1][y] == 0)
			{
				t.t[x+1][y] = 61; //missile right
				Draw(x+1, y);
				if (x+1 == t.posx && y == t.posy)
					DrawUs();
			}
		}
		if ((int)x-1 >=0)
		{
			if (t.t[x-1][y] == 0)
			{
				t.t[x-1][y] = 62; //missile left
				Draw(x-1, y);
				if (x-1 == t.posx && y == t.posy)
					DrawUs();
			}
		}
		if (y+1 < DIMY)
		{
			if (t.t[x][y+1] == 0)
			{
				t.t[x][y+1] = 70; //missile bottom
				Draw(x, y+1);
				if (x == t.posx && y+1 == t.posy)
					DrawUs();
			}
		}
		if ((int)y-1 >=0)
		{
			if (t.t[x][y-1] == 0)
			{
				t.t[x][y-1] = 63; //missile top
				Draw(x, y-1);
				if (x == t.posx && y-1 == t.posy)
					DrawUs();
			}
		}
	}
}

void Game::DoGrenade (Uint16 x, Uint16 y)
{
	if ((x-t.posx)*(x-t.posx) + (y-t.posy)*(y-t.posy) < DHEAR2)
	{
		sounds.Play(20);
	}
	int dx,dy;
	t.t[x][y] = 224; //hole
	for (dx=-1;dx<=1;dx++)
	{
		for (dy=-1;dy<=1;dy++)
		{
			if (x+dx >= 0 && x+dx < DIMX && y+dy >= 0 && y+dy < DIMY)
			{
				if (t.t[x+dx][y+dy] == 300) //we won !!!
				{
					Animate(x+dx, y+dy, 27);
					win = won;
					quit = true;
				}else
				{
					if (!t.t[x+dx][y+dy] || (!dx && !dy))
					{
						//printf("grenade action at (%d,%d)...\n",x+dx,y+dy);
						Animate(x+dx, y+dy, 29);
					}
					DoBullet(x+dx, y+dy);
				}
			}
		}
	}
}

void Game::DoExplosive (Uint16 x, Uint16 y) //Start an explosive located at (x, y)
{
	if (t.t[x][y] == 151) //time bomb
	{
		Animate(x, y, 11);
		return;
	}
	//printf("Starting an explosive at (%d,%d)...\n",x,y);
	if ((x-t.posx)*(x-t.posx) + (y-t.posy)*(y-t.posy) < DHEAR2)
	{
		sounds.Play(16); //plays the sound
	}
	int dx,dy;
	Animate(x , y, 33); //center of explosion
	for (dx=-1;dx<=1;dx++)
	{
		for (dy=-1;dy<=1;dy++)
		{
			if (x+dx >= 0 && x+dx < DIMX && y+dy >= 0 && y+dy < DIMY)
			{
				if (t.t[x+dx][y+dy] == 300) //we win
				{
					Animate(x+dx, y+dy, 27);
					win = won;
					quit = true;
				}else
				{
					if (!t.t[x+dx][y+dy])
					{
						//printf("explosion at (%d,%d)\n",x+dx,y+dy);
						Animate(x+dx, y+dy, 28);
					}
					/*else
					{
						if (r[t.t[x+dx][y+dy]].is(explosive))
							DoBullet(x+dx, y+dy);
					}*/
				}
			}
		}
	}
}

void Game::DrawBall (Uint16 x, Uint16 y)
{
	SDL_Rect r;
	r.x = x-1 - view.x + pview.x;
	r.y = y-1 - view.y + pview.y;
	r.w = 3;
	r.h = 3;
	if (r.x >= pview.x-3 && r.y >= pview.y-3 && r.x < pview.x + pview.w + 3 && r.y < pview.y + pview.h + 3)
	{
		if (r.x < pview.x)
			r.x = pview.x;
		if (r.y < pview.y)
			r.y = pview.y;
		if (r.x + r.w >= pview.x + pview.w)
			r.w = pview.x + pview.w - r.x - 1;
		if (r.y + r.h >= pview.y + pview.h)
			r.h = pview.y + pview.h - r.y - 1;
		if ((Uint16)r.w > 5 || (Uint16)r.h > 5)
			return;
		SDL_FillRect (screen, &r, SDL_MapRGB(screen->format, 250, 200, 0));
	}
}

void Game::PutRed (Uint8 red)
{
	if (!red)
		red = 1;
	SDL_Surface *s = screen;
	Uint32 *ptr = (Uint32 *)s->pixels+pview.y*s->pitch/4+pview.x;
	int i,m;
	Uint8 r,g,b;
	m = s->pitch/4;
	Uint32 x,y;
	for (y=0;y<NY*CY;y++)
	{
		for (x=0;x<NX*CX;x++)
		{
			SDL_GetRGB(*ptr,s->format,&r,&g,&b);
			if ((g == 0 && b == 0 && r != 0) || (g == 0 && b == 1 && r == 0))
			{
				*ptr = SDL_MapRGB(s->format, red, 0, 0);
				SDL_GetRGB(*ptr,s->format,&r,&g,&b);
			}
			ptr++;
		}
		ptr += (m-CX*NX);
	}
}
/*
 void Game::PutRed (Uint8 red)
 {
	 if (!red)
		 red = 1;
	 SDL_Surface *s = screen;
	 Uint8 *ptr = (Uint8 *)s->pixels+pview.y*s->pitch+pview.x*4;
	 int i,m;
	 Uint8 r,g,b;
	 m = s->pitch/4;
	 Uint32 x,y;
	 for (y=0;y<NY*CY;y++)
	 {
		 for (x=0;x<NX*CX;x++)
		 {
			 //SDL_GetRGB(*ptr,s->format,&r,&g,&b);
			 r = *ptr; ptr++;
			 g = *ptr; ptr++;
			 b = *ptr; ptr++;
			 if ((g == 0 && b == 0 && r != 0) || (g == 0 && b == 1 && r == 0))
			 {
				 *ptr = SDL_MapRGB(s->format, red, 0, 0);
				 SDL_GetRGB(*ptr,s->format,&r,&g,&b);
			 }
			 ptr++;
		 }
		 ptr += (m-CX*NX)*4;
	 }
 }
 */

//thread that manage the balls, flashing red and the update of the screen
int DoBullets (void *data)
{
	int i;
	Game *j = (Game *)data;
	Uint32 t,rt;
	int x,y,dx,dy;
	rt = SDL_GetTicks();
	Sint8 tmp;
	j->InitBalls ();
	int red = 0;
	int vred = 10;
	Uint32 lag = 0;
	do
	{
		if (j->paused)
		{
			j->DoPause();
			rt = SDL_GetTicks();
		}
		t = SDL_GetTicks();
		if ((int)t-rt < TBALLS)
		{
			SDL_Delay(TBALLS-(t-rt));
		}else
		{
			if (lag)
				printf("accumulated lag !!! (lag = %d)...\n",lag);
			lag = (t-rt)/TBALLS;
		}
		rt = SDL_GetTicks();//t;//+TBALLS;
		if (!lag)
		{
			//update what is on screen
			SDL_BlitSurface(j->board, &j->view, j->screen, &j->pview);
		}
		// draw balls
		for (i=0;i<NBMAX;i++)
		{
			if (j->b[i].used)
			{
				//see if the ball lands on a new box
				x = (int)((j->b[i].x+j->b[i].vx)/CX);
				y = (int)((j->b[i].y+j->b[i].vy)/CY);
				if ((Uint16)x >= DIMX || (Uint16)y >= DIMY)
				{
					j->b[i].used = false;
					continue;
				}
				if (x == j->t.posx && y == j->t.posy)
				{//ball hits us
					j->Wasted ();
				}
				if ((int)(j->b[i].x/CX) != x || (int)(j->b[i].y/CY) != y || j->b[i].react == 0) //new box
				{
					j->b[i].react = 1;
					if (x < 0 || y < 0 || x >= DIMX || y >= DIMY)
					{//ball out of field
						//printf("out !\n");
						j->b[i].used = false;
					}else
					{
						if (j->r[j->t.t[x][y]].is(react_ball))
						{//ball hits object
							j->DoBullet(x, y);
							j->b[i].used = false;
						}
						if (j->r[j->t.t[x][y]].is(stop_ball))
						{//ball stops
							j->b[i].used = false;
							//printf("vx=%d, vy=%d.\n",j->b[i].vx,j->b[i].vy);
							if (j->b[i].vx != 0 && j->b[i].vy != 0) //diagonal ball
							{
								//printf("diagonal bullet.../n");
								if (j->r[j->t.t[x][y]].is(wall))
								{
									//printf("...and wall ...\n");
									int rx,ry;
									rx = (int)(j->b[i].x/CX);
									ry = (int)(j->b[i].y/CY);
									//bounce
									if (x!=rx && y!=ry)
									{
										if (j->r[j->t.t[rx][y]].is(wall))
											j->b[i].vy = -j->b[i].vy;
										else
										{
											j->b[i].vx = -j->b[i].vx;
											if (j->r[j->t.t[rx][y]].is(wall))
												j->b[i].vy = -j->b[i].vy;
										}
									}else
									{
										if (x!=rx)
											j->b[i].vx = -j->b[i].vx;
										else
											j->b[i].vy = -j->b[i].vy;
									}
									/*
									j->b[i].vx = (2*(rx-x)-1)*VBALL;
									j->b[i].vy = (2*(ry-y)-1)*VBALL;
									 */
									j->b[i].used = true;
								}
							}
						}
					}
				}
				if (((int)((j->b[i].x+sign(j->b[i].vx)*CX/2)/CX) != x || (int)((j->b[i].y+sign(j->b[i].vy)*CY/2)/CY) != y) && j->b[i].react != 2) //middle box
				{
					j->b[i].react = 2;
					if (j->r[j->t.t[x][y]].is(act_ball))
					{//object acts on the bullet
						switch (j->t.t[x][y])
						{
							case 284: //anti-diagonal mirror
							case 203:
								tmp = j->b[i].vy;
								j->b[i].vy = -j->b[i].vx;
								j->b[i].vx = -tmp;
								break;
							case 294: //diagonal mirror
							case 213:
								tmp = j->b[i].vy;
								j->b[i].vy = j->b[i].vx;
								j->b[i].vx = tmp;
								break;
							case 304: //horizontal mirror
							case 193:
								j->b[i].vx = -j->b[i].vx;
								break;
							case 314: //vertical mirror
							case 183:
								j->b[i].vy = -j->b[i].vy;
								break;
							case 291: //horizontal and vertical mirror
								j->b[i].vx = -j->b[i].vx;
								j->b[i].vy = -j->b[i].vy;
								break;
							case 301: //multiple shooter
								j->b[i].used = false;
								//j->sounds.Play(5);
								j->Bullet(x, y, 1, 0);
								j->Bullet(x, y, -1, 0);
								j->Bullet(x, y, 0, 1);
								j->Bullet(x, y, 0, -1);
								break;
							case 285: //cannon left
								j->sounds.Play(5);
								j->Bullet(x, y, -1, 0);
								break;
							case 295: //cannon right
								j->sounds.Play(5);
								j->Bullet(x, y, 1, 0);
								break;
							case 305: //cannon bottom
								j->sounds.Play(5);
								j->Bullet(x, y, 0, 1);
								break;
							case 315: //cannon top
								j->sounds.Play(5);
								j->Bullet(x, y, 0, -1);
								break;
							case 287: //cannon bottom special
								j->sounds.Play(5);
								j->Bullet(x, y, 0, 1);
								j->b[i].used = false;
								break;
							case 280: //green diagonal shooter (shooting in all directions)
								j->sounds.Play(5);
								j->Bullet(x, y, -1, -1);
								j->Bullet(x, y, -1, 1);
								j->Bullet(x, y, 1, -1);
								j->Bullet(x, y, 1, 1);
								j->b[i].used = false;
								break;
							case 290: //shooter yellow diagonal (shooting a bullet at us)
								if (x < j->t.posx)
									dx = 1;
								else
									dx = -1;
								if (y < j->t.posy)
									dy = 1;
								else
									dy = -1;
								j->Bullet(x, y, dx, dy);
								j->b[i].used = false;
								break;
						}
					}
				}
				//move the ball
				j->b[i].x += j->b[i].vx;
				j->b[i].y += j->b[i].vy;
				//redraws the ball
				j->DrawBall(j->b[i].x, j->b[i].y);
			}
		}
		//apply the red flashing
		if (!lag)
		{
			if (red > 255)
			{
				red = 255;
				vred = -abs(vred);
			}
			if (red < -300)
			{
				red = -300;
				vred = abs(vred);
			}
			if (RED_FLASH)
			{
				if (red >= 0)
					j->PutRed(red);
			}
			red += vred;
		}
		//display
		if (!lag)
			SDL_Flip(j->screen);
		if (lag)
			lag--;
	}while(!j->quitanim);
	printf("End of DoBullets !\n");
	return 0;
}

//thread that updates the view
int DoView (void *data)
{
	Game *j = (Game *)data;
	j->doviewrun = true;
	SDL_Rect nview; //new view area
	bool update = true;
	do
	{
		if (j->paused)
		{
			j->DoPause();
		}
		SDL_Delay(20);
		if (j->updateview)
		{
			j->updateview = false;
			nview = j->view;
			//to determine where to go
			int dx = 0, dy = 0; //direction of the movement to perform
			if (j->vposx*CX < j->view.x + 2*CX)
				dx = -1;
			if (j->vposx*CX >= j->view.x + j->view.w - 2*CX)
				dx = 1;
			if (j->vposy*CY < j->view.y + 2*CY)
				dy = -1;
			if (j->vposy*CY >= j->view.y + j->view.h - 2*CY)
				dy = 1;
			nview.x += dx*3*CX;
			nview.y += dy*3*CY;
			if (nview.x < 0)
				nview.x = 0;
			if (nview.y < 0)
				nview.y = 0;
			if (nview.x + nview.w >= CX*DIMX)
				nview.x = CX*DIMX - nview.w;
			if (nview.y + nview.h >= CY*DIMY)
				nview.y = CY*DIMY - nview.h;
			if (nview.x == j->view.x && nview.y == j->view.y)
			{
				//j->doviewrun = false;
				//return 0;
				continue;
			}
			//performs the movement
			j->MoveViewTo2(nview);
		}
	}while(!j->quit && !j->quitanim);
	j->doviewrun = false;
	printf("End of DoView...\n");
}
/*
int DoView (void *data)
{
	int dx,dy; //movement to perform
	float vx, vy; //movement speed
	float x,y; //current position
	int fx,fy; //final position
	Game *j = (Game *)data;
	if (j->doviewrun)
	{
		j->updateview = true;
		return 0;
	}
	j->doviewrun = true;
	j->updateview = true;
	
	x = j->view.x;
	y = j->view.y;
	
	//perform the movement
	bool reupdate = false;
	Uint32 t,rt;
	int i;
	rt = SDL_GetTicks();
	Uint16 m = NVIEW;
	for (i=0;i<m;i++)
	{
		if (j->updateview)
		{
			if (reupdate)
			{
				//vx = dx*CX/(3*NMUS); ///not great !!!!
				//vy = dy*CY/(3*NMUS);
				i-=NMUS;
			}else
			{
				dx = 0;
				dy = 0;
				if (j->t.posx*CX < j->view.x + 2*CX)
					dx = -3;
				if (j->t.posx*CX >= j->view.x + j->view.w - 2*CX)
					dx = 3;
				if (j->t.posy*CY < j->view.y + 2*CY)
					dy = -3;
				if (j->t.posy*CY >= j->view.y + j->view.h - 2*CY)
					dy = 3;
				if (!dx && !dy)
					break;
				if (abs(j->t.posx-(int)((j->view.x+j->view.w/2)/CX))+abs(j->t.posy-(int)((j->view.y+j->view.h/2)/CY)) >= (NX+NY)/2) //check whether we have to move a lot or not
				{
					Pos p = j->GoodView ();
					fx = p.x/CX; //we move a lot
					fy = p.y/CY;
					m = 3*NVIEW;
				}else
				{
					fx = (int)((x+dx*CX)/CX); //we move normally
					fy = (int)((y+dy*CY)/CY);
					m = NVIEW;
				}
				vx = (fx*CX-x)/m;
				vy = (fy*CY-y)/m;
				i = 0;
			}
			//printf("x=%d, y=%d, fx=%d, fy=%d, vx=%d, vy=%d\n",(int)x,(int)y,(int)fx,(int)fy,vx,vy);
			j->updateview = false;
			reupdate = true;
		}
		t = SDL_GetTicks();
		if ((int)t-rt < TVIEW)
			SDL_Delay(TVIEW-(t-rt));
		//else
		//	printf("DoView running late...\n");
		rt = SDL_GetTicks();//t;//+TVIEW;
		x += vx;
		y += vy;
		j->view.x = (int)x;
		j->view.y = (int)y;
		if (x < 0)
		{
			//printf("edge !\n");
			j->view.x = 0;
			dx = 0;
		}
		if (y < 0)
		{
			//printf("edge !\n");
			j->view.y = 0;
			dy = 0;
		}
		if (x >= CX*DIMX - j->view.w)
		{
			//printf("edge !\n");
			j->view.x = CX*DIMX - j->view.w;
			dx = 0;
		}
		if (y >= CY*DIMY - j->view.h)
		{
			//printf("edge !\n");
			j->view.y = CY*DIMY - j->view.h;
			dy = 0;
		}
		if (j->quitanim || (!dx && !dy))
			break;
	}
	j->doviewrun = false;
	//printf("End of DoView !\n");
	return 0;
}
*/

void Game::UpdateView (Uint16 x, Uint16 y) //updates the field of view
{
	//launches the thread that updates the field of view
	vposx = x;
	vposy = y;
	updateview = true;
	if (!doviewrun)
	{
		doviewrun = true;
		printf("call DoView...\n");
		SDL_CreateThread(DoView, this);
	}
}

//thread responsible for animations
int DoAnim (void *data)
{
	int x,y,dx,dy;
	Uint16 n;
	Uint32 t,rt;
	Game *j = (Game *)data;
	rt = SDL_GetTicks();
	bool ok;
	Pos p;
	do
	{
		if (j->paused)
		{
			j->DoPause();
		}
		t = SDL_GetTicks();
		if ((int)t-rt < TANIM)
			SDL_Delay(TANIM-(t-rt));
		//else
		//	printf("DoAnimation running late...\n");
		rt = SDL_GetTicks();//t;//+TANIM;
		for (y=0;y<DIMY;y++)
		{
			for (x=0;x<DIMX;x++)
			{
				if (x == j->t.posx && y == j->t.posy)
				{
					if (j->r[j->t.t[x][y]].is(enemy))
					{
						j->Wasted();
					}
				}
				n = j->t.a[x][y].n;
				if (!n)
				{
					//conveyor belt
					if (j->r[j->t.t[x][y]].is(conveyor_belt))
					{
						j->t.a[x][y].t++;
						if (j->t.a[x][y].t >= TCONVEYOR_BELT)
						{
							j->t.a[x][y].t = 0;
							j->t.t[x][y] += 10;
							if (j->t.t[x][y] >= 249)
							{
								j->t.t[x][y] = 209;
							}
							j->Draw(x,y);
							if (j->t.posx == x && j->t.posy == y)
								j->DrawUs();
						}
					}
					//enemy
					if (j->t.t[x][y] == 240) //enemy horizontal left
					{
						ok = false;
						if (x > 0)
						{
							if (j->t.t[x-1][y] == 0 && j->t.a[x-1][y].n == 0)
							{
								j->Animate(x-1, y, 36);
								j->Animate(x, y, 37);
								ok = true;
							}
						}
						if (!ok)
						{
							if (x < DIMX-1)
							{
								if (j->t.t[x+1][y] == 0 && j->t.a[x+1][y].n == 0)
								{
									j->Animate(x, y, 34);
									j->Animate(x+1, y, 35);
								}
							}
						}
					}
					if (j->t.t[x][y] == 328) //enemy horizontal right
					{
						ok = false;
						if (x < DIMX-1)
						{
							if (j->t.t[x+1][y] == 0 && j->t.a[x+1][y].n == 0)
							{
								j->Animate(x, y, 34);
								j->Animate(x+1, y, 35);
								ok = true;
							}
						}
						if (!ok)
						{
							if (x > 0)
							{
								if (j->t.t[x-1][y] == 0 && j->t.a[x-1][y].n == 0)
								{
									j->Animate(x-1, y, 36);
									j->Animate(x, y, 37);
								}
							}
						}
					}
					if (j->t.t[x][y] == 322) //enemy vertical up
					{
						ok = false;
						if (y > 0)
						{
							if (j->t.t[x][y-1] == 0 && j->t.a[x][y-1].n == 0)
							{
								j->Animate(x, y-1, 38);
								j->Animate(x, y, 39);
								ok = true;
							}
						}
						if (!ok)
						{
							if (y < DIMY-1)
							{
								if (j->t.t[x][y+1] == 0 && j->t.a[x][y+1].n == 0)
								{
									j->Animate(x, y, 40);
									j->Animate(x, y+1, 41);
								}
							}
						}
					}
					if (j->t.t[x][y] == 340) //enemy vertical down
					{
						ok = false;
						if (y < DIMY-1)
						{
							if (j->t.t[x][y+1] == 0 && j->t.a[x][y+1].n == 0)
							{
								j->Animate(x, y, 40);
								j->Animate(x, y+1, 41);
								ok = true;
							}
						}
						if (!ok)
						{
							if (y > 0)
							{
								if (j->t.t[x][y-1] == 0 && j->t.a[x][y-1].n == 0)
								{
									j->Animate(x, y-1, 38);
									j->Animate(x, y, 39);
								}
							}
						}
					}
					//onda-mania
					if (j->r[j->t.t[x][y]].is(onda_mania))
					{
						dx = dy = 0;
						if (j->t.t[x][y] == 282) //try to go right
							dx = 1;
						if (j->t.t[x][y] == 273) //try to go left
							dx = -1;
						if (j->t.t[x][y] == 247) //try to go down
							dy = 1;
						if (j->t.t[x][y] == 274) //try to go up
							dy = -1;
						if ((Uint16)(x+dx) <= DIMX && (Uint16)(y+dy) <= DIMY)
						{
							if (x+dx == j->t.posx && y+dy == j->t.posy)
							{
								j->Wasted();
							}else
							{
								if (!j->t.t[x+dx][y+dy] && !j->t.a[x+dx][y+dy].n)
								{//the onda-mania can move forward
									int na,na2;
									switch (j->t.t[x][y])
									{
										case 282: //go right
											na = 23;
											na2 = 24;
											break;
										case 273: //go left
											na = 20;
											na2 = 19;
											break;
										case 274: //go up
											na = 22;
											na2 = 21;
											break;
										case 247: //go down
											na = 25;
											na2 = 26;
											break;
									}
									j->Animate(x, y, na);
									j->Animate(x+dx, y+dy, na2);
									n = j->t.a[x][y].n;
									dx = dy = 0;
								}
							}
						}
						if (dx != 0 || dy != 0)
						{
							//printf("%d : trying a new direction... ",j->t.t[x][y]);
							//the onda-mania will try a new direction
							switch (j->t.t[x][y])
							{
								case 282: //trying to go right
									j->t.t[x][y] = 247; //go up
									break;
								case 273: //trying to go left
									j->t.t[x][y] = 274; //go down
									break;
								case 247: //trying to go down
									j->t.t[x][y] = 273; //go left
									break;
								case 274: //trying to go up
									j->t.t[x][y] = 282; //go right
									break;
							}
							//printf(" -> %d !\n",j->t.t[x][y]);
						}
					}
				}
				if (n)
				{
					if ((n == 32 || n == 42) && j->t.a[x][y].t == j->a[n].t-1) //rail that switches off //infinite battery
					{
						//printf("rail or battery...\n");
						/*
						//printf("rail (%d,%d) switches on...\n",x, y);
						j->ActivateRail(x, y);
						j->Draw(x, y);
						if (x == j->t.posx && y == j->t.posy)
							j->DrawUs();
						if (x == j->t.posx && y == j->t.posy && !j->moveus)
						{
							j->moveus = true;
							j->t.dir = down;
							j->sounds.Play(21); //pulse
						}
						 */
						if (n == 32) //rail gets deactivated
						{
							j->DeactivateRail(x, y);
							j->Draw(x, y);
							if (x == j->t.posx && y == j->t.posy)
								j->DrawUs();
							p = GetRailDir (j->t.t[x][y]);
						}else
						{
							p.x = 0;
							p.y = 1;
						}
						if ((Uint16)(x+p.x) < DIMX && (Uint16)(y+p.y) < DIMY)
						{
							//printf("Green rail switching on (n=%d, x=%d, y=%d)...\n",j->t.a[x][y].n, x+p.x, y+p.y);
							if (j->r[j->t.t[x+p.x][y+p.y]].is(rail))
							{
								//Pos p2 = GetRailDir2 (j->t.t[x+p.x][y+p.y], GetDir(p.x, p.y));
								//if (p2.x || p2.y) //verifies that the track goes in the same direction
								{
									j->ActivateRail(x+p.x, y+p.y);
									j->Draw(x+p.x, y+p.y);
									if (x+p.x == j->t.posx && y+p.y == j->t.posy)
										j->DrawUs();
									if (x+p.x == j->t.posx && y+p.y == j->t.posy && !j->moveus)
									{
										j->moveus = true;
										j->t.dir = down;
										j->sounds.Play(21); //pulse
									}
									j->Animate(x+p.x, y+p.y, 32); //green rail switching off
								}
							}
						}
						if (n == 42)
							j->Animate(x, y, 42); //relaunch the infinite battery
					}
					/*
					if (n == 32 && j->t.a[x][y].t >= j->a[n].t-1) //rail gets deactivated
					{
						//printf("rail (%d,%d) switches off...\n",x, y);
						j->DeactivateRail(x, y);
						j->t.a[x][y].t = 0; //j->t.a[x][y].t%j->a[n].t;
						j->Draw(x, y);
						if (x == j->t.posx && y == j->t.posy)
							j->DrawUs();
					}
					 */
					if (n == 11 && j->t.a[x][y].t == j->a[n].n*j->a[n].t-1) //time bomb
					{
						j->DoExplosive(x, y);
					}
					if (n == 28 && j->t.a[x][y].t == 3) //explosion
					{
						//check whether it kills us
						j->DoBullet(x, y);
					}
					if (n == 33 && j->t.a[x][y].t == 3) //center of bomb explosion
					{
						//starts nearby things, if any
						for (dx=-1;dx<=1;dx++)
						{
							for (dy=-1;dy<=1;dy++)
							{
								if ((Uint16)(x+dx) < DIMX && (Uint16)(y+dy) < DIMY)
								{
									j->DoBullet(x+dx, y+dy);
								}
							}
						}
					}
					if (n == 16 && j->t.a[x][y].t == j->a[n].n*j->a[n].t-1) //virus
					{
						for (dx=-1;dx<=1;dx++)
						{
							for (dy=-1;dy<=1;dy++)
							{
								if (x+dx >= 0 && x+dx < DIMX && y+dy >= 0 && y+dy < DIMY)
								{
									if (!j->t.t[x+dx][y+dy])
										j->Animate(x+dx, y+dy, 16);
								}
							}
						}
					}
					if (n == 30 && j->t.a[x][y].t == j->a[n].n*j->a[n].t-1)
					{
						j->OpenDoors();
					}
					//printf("Animation %d : t = %d...\n",n,j->a[n].t);
					if (j->t.a[x][y].t % j->a[n].t == 0 && j->a[n].n) //new image
					{
						j->t.t[x][y] = j->a[n].l[j->t.a[x][y].t / j->a[n].t];
						j->Draw(x,y);
						if (j->t.posx == x && j->t.posy == y)
						{
							if (n != 27 && (n != 18 || (n==18 && (Uint16)(j->t.a[x][y].t/j->a[n].t) == j->a[n].n-1))) //death and new life
								j->DrawUs();
						}
					}
					j->t.a[x][y].t++;
					if (j->t.a[x][y].t >= j->a[n].n*j->a[n].t && j->a[n].n)
					{
						j->t.t[x][y] = j->a[n].l[j->a[n].n-1];
						j->t.a[x][y].init();
					}
				}
			}
		}
	}while(!j->quitanim);
	printf("End of DoAnim !!!\n");
	return 0;
}

void Game::Animate (Uint16 x, Uint16 y, Uint16 n) //start the animation n (x,y)
{
	//printf("Starting animation %d at (%d, %d) ...\n",n,x,y);
	if (n >= NA)
	{
		fprintf(stderr, "Error : animation %d doesn't exist !\n",n);
		return;
	}
	//printf("sound %d\n",a[n].s);
	//printf("time %d\n",a[n].t);
	if (a[n].s)
	{
		//verifies that it is not too far away to hear
		if ((x-t.posx)*(x-t.posx) + (y-t.posy)*(y-t.posy) < DHEAR2)
			sounds.Play(a[n].s); //plays the sound
	}
	if (t.a[x][y].n != 27)
	{
		t.a[x][y].n = n;
		t.a[x][y].t = 0;
	}
}

void Game::Update ()
{
	
	SDL_BlitSurface(board, &view, screen, &pview);
	SDL_Flip(screen);
}

inline void Game::DrawUs ()
{
	if (quit && win == wasted)
		return;
	SDL_Rect src, pos;
	src.x = CX*(t.dir);
	src.y = CY*(t.tdir);
	src.w = CX;
	src.h = CY;
	pos.x = t.posx*CX + pr.x;
	pos.y = t.posy*CY + pr.y;
	SDL_BlitSurface(im, &src, board, &pos);
}

inline void Game::DrawGround (Uint16 x, Uint16 y)
{
	SDL_Rect src, pos;
	src.x = CX*(t.ground[x][y]%10);
	src.y = CY*(t.ground[x][y]/10);
	src.w = CX;
	src.h = CY;
	pos.x = x*CX;
	pos.y = y*CY;
	SDL_BlitSurface(im, &src, board, &pos);
}

inline void Game::Draw(Uint16 x, Uint16 y)
{
	SDL_Rect src, pos;
	if (!t.ground[x][y])
		t.ground[x][y] = 41;
	src.x = CX*(t.ground[x][y]%10);
	src.y = CY*(t.ground[x][y]/10);
	src.w = CX;
	src.h = CY;
	pos.x = x*CX;
	pos.y = y*CY;
	SDL_BlitSurface(im, &src, board, &pos); //ground
	if (t.t[x][y]%10 >= 4 || t.t[x][y]/10 >= 4)
	{
		src.x = CX*(t.t[x][y]%10);
		src.y = CY*(t.t[x][y]/10);
		SDL_BlitSurface(im, &src, board, &pos); //box content
	}
}

void Game::Draw()
{
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			Draw(x,y);
		}
	}
	DrawUs();
}

void Game::initRail (Uint16 x, Uint16 y, Uint16 time, Uint8 dir) //initializes the rail (x, y) and the following (useless)
{
	if (x >= DIMX || y >= DIMY)
		return;
	if (!r[t.t[x][y]].is(rail))
		return;
	Pos p;
	/*
	switch (dir)
	{
		case left:
			printf("left !\n");
			break;
		case right:
			printf("right !\n");
			break;
		case down:
			printf("down !\n");
			break;
		case up:
			printf("up !\n");
			break;
	}
	 */
	p = GetRailDir(t.t[x][y], dir);
	t.a[x][y].n = 32; //green rail that lights
	t.a[x][y].t = time;
	//printf("towards (%d,%d) ...\n",p.x,p.y);
	
	DeactivateRail(x, y);
	if (!p.x && !p.y)
	{
		//printf("Wrong direction !\n");
		return;
	}
	time -= TRAIL;
	if ((Uint32)time >= a[32].t)
		time = a[32].t-1;
	initRail(x+p.x, y+p.y, time, GetDir(p.x, p.y));
}

void Game::DeactivateRails ()
{
	Uint16 x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (r[t.t[x][y]].is(rail))
			{
				DeactivateRail(x, y);
			}
		}
	}
}

void Game::initTrain () //initiates automatic rails
{
	Uint16 x,y;
	DeactivateRails();
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (t.t[x][y] == 173) //infinite battery
			{
				//printf("Infinite battery found at (%d,%d) ...\n",x,y);
				//initRail(x, y+1, 0, down);
				Animate(x, y, 42);
			}
		}
	}
}

void Game::InitBalls ()
{
	int i;
	for (i=0;i<NBMAX;i++)
	{
		b[i].init();
	}
}

void Game::init ()
{
	char buf[256];
	//initialize SDL
	//SDL_errorcode err;
#if HSOUND
	if (SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO))
	{
		fprintf(stderr,"Initialize SDL error : %s\n",SDL_GetError());
		exit(EXIT_FAILURE);
	}
#else
	if (SDL_Init(SDL_INIT_VIDEO))
	{
		fprintf(stderr,"Initialize SDL error : %s\n",SDL_GetError());
		exit(EXIT_FAILURE);
	}
#endif
#if TTF
	//initialize ttf
	//printf("Initialise TTF...\n");
	if (TTF_Init() < 0)
	{
		fprintf(stderr,"Initialize TTF error !!!\n");
		exit(EXIT_FAILURE);
	}
	//printf("Loading the font...\n");
	MakeName("arial.ttf", buf, RESOURCES, 256);
	font = TTF_OpenFont(buf, 20);
	if (!font)
	{
		printf("Font not found !\n");
		exit(EXIT_FAILURE);
	}
#endif
	//Opening the window
	screen = SDL_SetVideoMode(SCREENX, SCREENY, 32, SDL_HWSURFACE);
    if (screen == NULL)
    {
        fprintf(stderr, "Unable to load video mode : %s\nq", SDL_GetError());
        exit(EXIT_FAILURE);
    }
#if HSOUND
	//initialize sound
	sounds.Init();
#endif
	//Loads game images
	MakeName("images.bmp", buf, RESOURCES, 256);
	im = SDL_LoadBMP(buf);
	if (!im)
	{
		fprintf(stderr,"Error : Game images missing !\n");
	}
	SDL_SetColorKey(im, SDL_SRCCOLORKEY, SDL_MapRGB(im->format, 0, 0, 255)); //transparency
	//initialize field of view
	view.x = 0;
	view.y = 0;
	view.w = NX*CX;
	view.h = NY*CY;
	pview = view;
	pview.x = (SCREENX-view.w)/2;
	pview.y = (SCREENY-view.h)/2;
	doviewrun = false;
	moveus = false;
	canmove = false;
	paused = false;
	//create the image of the complete level
	board = SDL_CreateRGBSurface(SDL_HWSURFACE, CX*DIMX, CY*DIMY, 32, 0, 0, 0, 0);
	if (!board)
	{
		fprintf(stderr, "Unable to create the playing surface !\n");
	}
	//create the image of one box
	boximg = SDL_CreateRGBSurface(SDL_HWSURFACE, CX, CY, 32, 0, 0, 0, 0);
	if (!boximg)
	{
		fprintf(stderr, "Unable to create the surface of a box !\n");
	}
	//load the rules
	LoadRules(r);
	//load animations
	//printf("Loading animations...\n");
	LoadAnims(a);
	//initialize bullets
	InitBalls();
	//initializes the respawn point
	t.sx = t.sy = 0;
	//initializes the control buttons
	key[kleft] = SDLK_LEFT;
	key[kright] = SDLK_RIGHT;
	key[kup] = SDLK_UP;
	key[kdown] = SDLK_DOWN;
	key[kshoot] = SDLK_RETURN;
	key[ktake] = SDLK_SPACE;
	key[kteleport] = SDLK_SPACE;
	key[ksuicide] = SDLK_s;
	//load preferences (if any)
	LoadPrefs();
	//initialize data used by the editor to make adjacent tiles fit together
	w[0].init(0);
	w[1].init(1);
	w[2].init(2);
}

void Game::end ()
{
	//frees the image of the box
	SDL_FreeSurface(boximg);
	//frees the image of the complete level
	SDL_FreeSurface(board);
	//frees images of the game
	SDL_FreeSurface(im);
	//stop sound
	sounds.Stop();
#if TTF
	//close ttf
	TTF_CloseFont(font);
	TTF_Quit();
#endif
	//stop SDL
	SDL_Quit();
}

char inline dirx (Uint8 dir)
{
	switch (dir)
	{
		case left:
			return -1;
		case up:
		case down:
			return 0;
		case right:
			return 1;
	}
}

char inline diry (Uint8 dir)
{
	switch (dir)
	{
		case up:
			return -1;
		case left:
		case right:
			return 0;
		case down:
			return 1;
	}
}

void Game::Update_tdir ()
{
	if (r[t.object].is(missile))
	{
		switch (t.object)
		{
			case 61:
				t.tdir = right;
				break;
			case 62:
				t.tdir = left;
				break;
			case 63:
				t.tdir = up;
				break;
			case 70:
				t.tdir = down;
				break;
		}
	}else
		t.tdir = t.dir; //if no missile
}

Uint8 GetDir (int dx, int dy)
{
	if (dx == 1)
		return right;
	if (dx == -1)
		return left;
	if (dy == 1)
		return down;
	if (dy == -1)
		return up;
}

void Game::MoveUs (Uint8 dir)
{
	int dx = 0,dy = 0;
	bool train = false;
	//check whether we are on an active rail
	Uint16 c = t.t[t.posx][t.posy];
	if (r[c].is(rail_active))
	{
		//printf("Rail active !\n");
		train = true;
		Pos p = GetRailDir(c, dir);
		dx = p.x;
		dy = p.y;
		if (!dx && !dy)
			return;
		dir = GetDir(dx, dy);
		DeactivateRail(t.posx, t.posy);
		//printf("Move to (%d,%d) ...\n",dx,dy);
	}else
	{
		dx = dirx(dir);
		dy = diry(dir);
	}
	//update direction
	t.dir = dir;
	Update_tdir ();
	//check if we can move forward
	if (t.posx+dx < 0 || t.posx+dx >= DIMX || t.posy+dy < 0 || t.posy+dy >=DIMY)
		return;
	c = t.t[t.posx+dx][t.posy+dy];
	if (r[c].is(wall))
	{
		if (r[c].is(door)) //opening a one-way door
		{
			if ((dir == down && c == 81) || (dir == up && c == 82) || (dir == left && c == 83) || (dir == right && c == 84))
			{
				DoDoor(t.posx+dx, t.posy+dy);
				//printf("Opening one-way door (%d,%d)\n",t.posx+dx, t.posy+dy);
			}
		}
		if (!train || !r[c].is(rail))
			return;
	}
	if (r[t.t[t.posx][t.posy]].is(door_open))
	{//close a one-way door
		t.t[t.posx][t.posy] = 80 + (t.t[t.posx][t.posy]%10); //close the one-way door
	}
	//close nearby one-way doors
	int dx2,dy2;
	Uint32 x2,y2;
	for (dx2 = -1;dx2<=1;dx2++)
	{
		for (dy2 = -1; dy2 <= 1; dy2++)
		{
			if ((dx2 || dy2) && (dx2 != dx || dy2 != dy))
			{
				x2 = (Uint16)(t.posx+dx2);
				y2 = (Uint16)(t.posy+dy2);
				if (x2 < DIMX && y2 < DIMY)
				{
					if (r[t.t[x2][y2]].is(door_))//|| (t.a[x2][y2].n >= 1 && t.a[x2][y2].n <= 4)) //open one-way door
					{
						t.t[x2][y2] = 80 + (t.t[x2][y2]%10); //close the one-way door
						t.a[x2][y2].n = 0;
						Draw(x2, y2);
						//printf("closing one-way door (%d, %d) !\n",x2,y2);
					}
				}
			}
		}
	}
	//update the field of view
	UpdateView(t.posx+dx, t.posy+dy);
	sounds.Play(2); //movement sound
	Draw(t.posx, t.posy); //erase ourselves
	//movement animation
	int i;
	SDL_Rect src, pos, rc;
	rc.x = 0;
	rc.y = 0;
	rc.w = CX;
	rc.h = CY;
	src.x = CX*(t.dir);
	src.y = CY*(t.tdir);
	src.w = CX;
	src.h = CY;
	pos.x = t.posx*CX;
	pos.y = t.posy*CY;
	pos.w = CX;
	pos.h = CY;
	Uint8 *keystate;
	SDL_Event event;
	dx2 = dy2 = 0;
	Uint32 time,rtime;
	rtime = SDL_GetTicks();
	for (i=0;i<NMUS;i++)
	{
		if (invincible)
		{
			invincible--;
		}
		
		pos.x += (CX/NMUS)*dx;
		pos.y += (CY/NMUS)*dy;
		pr.x = pos.x-t.posx*CX;
		pr.y = pos.y-t.posy*CY;
		
		if (t.a[t.posx][t.posy].n)
			Draw(t.posx, t.posy);
		if (t.a[t.posx+dx][t.posy+dy].n)
			Draw(t.posx+dx, t.posy+dy);
		
		SDL_BlitSurface(board, &pos, boximg, &rc); //retains what is below
		SDL_BlitSurface(im, &src, board, &pos); //and draws us
		/*
		if (!doviewrun)
		{
			Update(); //updated
		}
		 */
		//
		time = SDL_GetTicks();
		if ((int)time-rtime < TMUS)
			SDL_Delay(TMUS-(time-rtime));
		//else
		//	printf("MoveUs running late...\n");
		//printf("time = %d, rtime = %d, realtime = %d\n",time,rtime,SDL_GetTicks());
		rtime = SDL_GetTicks();//time;//+TMUS;
		//
		rc.w = CX;
		rc.h = CY;
		SDL_BlitSurface(boximg, &rc, board, &pos); //returns
		if (train)
		{
			SDL_PollEvent(&event);
			//printf(".\n");
			keystate = SDL_GetKeyState (NULL);
			if (keystate[key[kup]])
			{
				//printf("top !!!\n");
				dy2 = -1;
			}
			if (keystate[key[kdown]])
			{
				//printf("down !!!\n");
				dy2 = 1;
			}
			if (keystate[key[kright]])
			{
				//printf("right !!!\n");
				dx2 = 1;
			}
			if (keystate[key[kleft]])
			{
				//printf("left !!!\n");
				dx2 = -1;
			}
		}
	}
	//we move to our final position
	t.posx += dx;
	t.posy += dy;
	pr.x = 0;
	pr.y = 0;
	//manages the acceleration
	if (tNMUS)
	{
		tNMUS--;
		if (!tNMUS)
		{
			NMUS = NMUS0;
		}
	}
	//starts objects that react when we pass over them
	if (r[c].is(react_on)) // || r[t.ground[t.posx][t.posy]].is(react_on))
	{
		//printf("react_on...\n");
		ReactOn(t.posx, t.posy);
	}
	//starts nearby stuff
	for (dx=-1;dx<=1;dx++)
	{
		for (dy=-1;dy<=1;dy++)
		{
			if (t.posx+dx >= 0 && t.posx+dx < DIMX && t.posy+dy >= 0 && t.posy+dy < DIMY)
			{
				if (r[t.t[t.posx+dx][t.posy+dy]].is(explosive))
					DoExplosive(t.posx+dx, t.posy+dy);
			}
		}
	}
	DrawUs();
	//train
	if (train && r[c].is(rail))
	{
		if (dx2 != 0 || dy2 != 0)
		{
			//printf("You want to get out (%d,%d) ?\n",dx2,dy2);
			if (!r[c].is(rail_wall))
			{
				//printf(" ...track permits...\n");
				if ((int)t.posx+dx2 >= 0 && t.posx+dx2 < DIMX && (int)t.posy+dy2 >= 0 && t.posy+dy2 < DIMY)
				{
					//printf("... you do not come out of the screen...\n");
					c = t.t[t.posx+dx2][t.posy+dy2];
					if (!r[c].is(wall))
					{
						//printf("...no wall...\n");
						//printf(" ...pulls out !!!\n");
						DeactivateRail (t.posx, t.posy); //to be on...
						MoveUs(GetDir(dx2, dy2));
						return;
					}
				}
			}
		}
		if (r[c].is(rail))
		{
			//printf("Rail reactivated...\n");
			ActivateRail (t.posx, t.posy);
			MoveUs(dir);
			return;
		}
	}
	if (r[c].is(conveyor_belt))
	{
		if (canmove)
			dir = t.dir;
		sounds.Play(9);
		MoveUs(dir);
		return;
	}
}

void pause (Uint32 t)
{
	Uint32 ti,rti;
	rti = SDL_GetTicks();
	SDL_Event event;
	for (;;)
	{
		SDL_Delay(10);
		SDL_PollEvent(&event);
		ti = SDL_GetTicks();
		if (ti-rti > t)
			break;
		if (event.type == SDL_QUIT || (event.type == SDL_KEYDOWN && event.key.keysym.sym == SDLK_ESCAPE))
		{
			SDL_PushEvent(&event);
			break;
		}
	}
}

void Game::Recenter ()
{
	Pos p = GoodView();
	doviewrun = false;
	view.x = p.x;
	view.y = p.y;
}

void Game::ChooseLevel (Uint16 *level)
{
	SDL_Event event;
	bool update = true;
	quit = false;
	do
	{
		if (update)
		{
			update = false;
			DispInfo(*level);
			t.sx = t.sy = 0;
#if MAC
			//UnsignedWide w1,w2,w3,w4,w5;
			//Microseconds(&w1);
#endif
			Load(*level);
#if MAC
			//Microseconds(&w2);
#endif
			pr.x = 0;
			pr.y = 0;
			StartView(); //put the field of view where it should be
#if MAC
			//Microseconds(&w3);
#endif
			Draw();
#if MAC
			//Microseconds(&w4);
#endif
			Update();
#if MAC
			//Microseconds(&w5);
			//printf("Load time : %d\n",w2.lo-w1.lo);
			//printf("StartView time : %d\n",w3.lo-w2.lo);
			//printf("Draw time : %d\n",w4.lo-w3.lo);
			//printf("Update time : %d\n",w5.lo-w4.lo);
#endif
		}
		if (SDL_WaitEvent(&event))
		{
			switch(event.type)
			{
				case SDL_QUIT:
					win = kabort;
					quit = true;
					break;
				case SDL_KEYDOWN:
					switch (event.key.keysym.sym)
					{
						case SDLK_ESCAPE:
							win = kabort;
							quit = true;
							break;
						case SDLK_LEFT:
							if (*level)
								*level = *level - 1;
							update = true;
							break;
							/*
						case SDLK_:
							*level = 0;
							update = true;
							break;
							*/
						case SDLK_F1:
							*level = 10;
							update = true;
							break;
						case SDLK_F2:
							*level = 20;
							update = true;
							break;
						case SDLK_F3:
							*level = 30;
							update = true;
							break;
						case SDLK_F4:
							*level = 40;
							update = true;
							break;
						case SDLK_F5:
							*level = 50;
							update = true;
							break;
						case SDLK_F6:
							*level = 60;
							update = true;
							break;
						case SDLK_F7:
							*level = 70;
							update = true;
							break;
						case SDLK_F8:
							*level = 90;
							update = true;
							break;
						case SDLK_F9:
							*level = 90;
							update = true;
							break;
						case SDLK_F10:
							*level = 100;
							update = true;
							break;	
						case SDLK_UP:
							*level = abs(rand())%101;
							update = true;
							break;
						case SDLK_RIGHT:
							*level = *level + 1;
							update = true;
							break;
					}
			}
		}
	}while(!quit);
}

void Game::DoPause ()
{
	do
	{
		SDL_Delay(10);
	}while(paused);
}

void Game::Pause ()
{
	SDL_Event event;
	paused = true;
	bool cont = true;
	do
	{
		SDL_WaitEvent(&event);
		switch(event.type)
		{
			case SDL_QUIT:
				cont = false;
				quit = true;
				break;
			case SDL_KEYDOWN:
				switch (event.key.keysym.sym)
				{
					case SDLK_ESCAPE:
						cont = false;
						break;
					case SDLK_p:
						cont = false;
						break;
				}
				break;
		}
	}while(cont);
	paused = false;
}

void Game::Play (Uint32 level)
{
	pr.x = 0;
	pr.y = 0;
	
	StartView(); //put the field of view where it should be
	Draw();
	//Update();
	
	quit = false;
	quitanim = false;
	moveus = false;
	canmove = false;
	win = wasted;
	t.dir = down;
	t.tdir = t.dir;
	invincible = 0;
	
	//start thread for animations
	SDL_Thread *tanim = SDL_CreateThread(DoAnim, this);
	//start thread for bullets
	SDL_Thread *tballs = SDL_CreateThread(DoBullets, this);
	
	SDL_PumpEvents();
	
	Animate(t.posx, t.posy, 18); //materialisation
	//DrawUs();
	
	SDL_Event event;
	//SDL_EnableKeyRepeat(10, 10);
	Uint32 time,rtime;
	Uint8 *keystate;
	rtime = SDL_GetTicks();
	
	pause(1200);
	
	do
	{
		time = SDL_GetTicks();
		if ((int)time-rtime < 50)
			SDL_Delay(50-(time-rtime));
		//else
		//	printf("Play running late...\n");
		rtime = SDL_GetTicks();//time+50;
		if (moveus)
		{
			MoveUs(t.dir);
			moveus = false;
		}
		if (invincible)
			invincible--;
		if (SDL_PollEvent(&event))
		{
			switch(event.type)
			{
				case SDL_QUIT:
					win = kabort;
					quit = true;
					break;
				case SDL_KEYDOWN:
					switch (event.key.keysym.sym)
					{
						case SDLK_ESCAPE:
							win = kabort;
							quit = true;
							break;
#if !FINAL || CHEATED
						case SDLK_a:
							quitanim = true;
							break;
						case SDLK_e:
							Edit(level);
							break;
						case SDLK_m:
							if (!invincible)
								invincible = INVINCIBLE;
							else
								invincible = (Uint16)(-1);
							break;
						case SDLK_u:
							level--;
							DispInfo(level);
							t.sx = t.sy = 0;
							Load(level);
							pr.x = 0;
							pr.y = 0;
							StartView(); //put the field of view where it should be
							Draw();
							break;
						case SDLK_l:
							level = abs(rand())%101;
							DispInfo(level);
							t.sx = t.sy = 0;
							Load(level);
							pr.x = 0;
							pr.y = 0;
							StartView(); //put the field of view where it should be
							Draw();
							break;
						case SDLK_o:
							level++;
							DispInfo(level);
							Load(level);
							pr.x = 0;
							pr.y = 0;
							StartView(); //put the field of view where it should be
							Draw();
							break;
#endif
						case SDLK_p:
							Pause();
							break;
						case SDLK_t:
							Recenter();
							break;
						case SDLK_i:
							printf("t[%d][%d] = %d\n",t.posx,t.posy,t.t[t.posx][t.posy]);
							printf("ground = %d\n",t.ground[t.posx][t.posy]);
							break;
						case SDLK_y:
							int count = 0;
							for (int x=0;x<DIMX;x++)
							{
								for (int y=0;y<DIMY;y++)
								{
									if (t.a[x][y].n == 42)
										count++;
								}
							}
							printf("Number of infinite batteries : %d\n",count);
							break;
					}
					if (event.key.keysym.sym == key[kteleport])
					{
						if (r[t.t[t.posx][t.posy]].is(teleporter)) //teleporter
						{
							Teleport(t.t[t.posx][t.posy]);
						}
					}
					if (event.key.keysym.sym == key[kshoot])
					{
						if (r[t.object].is(missile)) //firing missile
						{
							int dx,dy;
							dx = dy = 0;
							if (t.object == 61)
								dx = 1;
							if (t.object == 62)
								dx = -1;
							if (t.object == 70)
								dy = 1;
							if (t.object == 63)
								dy = -1;
							sounds.Play(3);
							Bullet(t.posx, t.posy, dx, dy);
							t.object = 0;
							OpenBarrier();
							Update_tdir ();
							DrawUs();
						}
					}
					if (event.key.keysym.sym == key[ktake]) //take an object
					{
						if (r[t.t[t.posx][t.posy]].is(object))
						{
							if (t.object)
							{//exchange two objects
								Uint16 tmp = t.object;
								t.object = t.t[t.posx][t.posy];
								t.t[t.posx][t.posy] = tmp;
							}else
							{//takes the object
								t.object = t.t[t.posx][t.posy];
								t.t[t.posx][t.posy] = 0;
								CloseBarrier();
							}
							Update_tdir ();
							DrawUs();
							sounds.Play(13);
						}else
						{
							if (!t.t[t.posx][t.posy])
							{
								if (t.object)
								{//drop the object
									t.t[t.posx][t.posy] = t.object;
									t.object = 0;
									Update_tdir ();
									DrawUs();
									sounds.Play(13);
									OpenBarrier();
								}
							}else
							{
								if (t.t[t.posx][t.posy] == 150 && t.object == 140) //lock && key
								{
									t.object = 0;
									Animate(t.posx, t.posy, 30);
									OpenBarrier();
								}
								if (r[t.t[t.posx][t.posy]].is(rail) && t.object == 163) //rail && battery
								{
									sounds.Play(21);
									ActivateRail(t.posx, t.posy);
									t.object = 0;
									//moveus = true;
									MoveUs(t.dir);
									OpenBarrier();
								}
							}
						}
					}
					if (event.key.keysym.sym == key[ksuicide])
					{
						invincible = 0;
						Wasted();
					}
					break;
			}
		}
		keystate = SDL_GetKeyState (NULL);
		if (keystate[key[kup]])
		{
			MoveUs(up);
		}
		if (keystate[key[kdown]])
		{
			MoveUs(down);
		}
		if (keystate[key[kright]])
		{
			MoveUs(right);
		}
		if (keystate[key[kleft]])
		{
			MoveUs(left);
		}
	}while(!quit);
	
	printf("end of game...\n");
	
	switch(win)
	{
		case wasted:
			printf("You died !\n");
			printf("pause 2 sec...\n");
			pause(2000);
			sounds.Play(23);
			printf("pause 3 sec...\n");
			pause(3000);
			break;
		case won:
			printf("You won !\n");
			printf("pause 2 sec...\n");
			pause(2000);
			sounds.Play(22);
			printf("pause 3 sec...\n");
			pause(3000);
			break;
	}
	
	printf("terminate thread ...\n");
	
	//quit thread for animations
	quitanim = true;
	pause(10*max(TANIM,max(TVIEW,TBALLS))); //wait a bit to allow the thread to stop
	SDL_PumpEvents();
	
	if (win != wasted)
	{
		t.sx = t.sy = 0; //resets the respawn point
	}
	printf("Game::Play end !\n");
}

int main(int argc, char *argv[])
{
	Game j;
	//Initalize the game
	j.init();
	//launch the main window
	j.Main();
	//quit the game
	j.end();
	return EXIT_SUCCESS;
}
