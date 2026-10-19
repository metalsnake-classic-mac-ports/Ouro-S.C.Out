
#define NBMAX	100 //maximum bullets

struct Ball
{
	Uint16 x,y; //position
	//SDL_Surface s;
	bool used; //Is the bullet active?
	
	void Do ();
};