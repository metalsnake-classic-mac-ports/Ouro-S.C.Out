#define DIMX	40 //dimensions of a level (in boxes)
#define DIMY	24

#define TRAIL	5 //time for the green to move from one box to the next (unused)

//possible directions
enum
{
	left = 0,
	up = 1,
	right = 2,
	down = 3
};

struct InfoAnim
{
	Uint16 n; //number of animations
	Uint32 t; //time
	
	void init ();
};

void InfoAnim::init ()
{
	n = 0;
	t = 0;
}

struct Level
{
	Uint8 ground[DIMX][DIMY];
	Uint16 t[DIMX][DIMY];
	InfoAnim a[DIMX][DIMY]; //used for animations (after time)
	Uint16	posx; //our position
	Uint16	posy;
	Uint16	sx; //respawn position
	Uint16	sy;
	Uint16	so; //object we hold when we respawn
	Uint8	dir; //our direction
	Uint8	tdir; //our direction of shooting
	Uint16	object;
	
	void Load (FILE *f); //load level from file
	void Save (FILE *f); //save level to file 
	void Load (Uint8 n); //load level n
	void Save (Uint8 n); //save level n
	void Load (const char *file); //load level from file
	void Save (const char *file); //save level to file
	void Default (); //default level (empty)
};

struct Pos
{
	Sint16 x;
	Sint16 y;
};

void Level::Default ()
{
	printf("Level::Default\n");
	int x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			t[x][y] = 0;
			ground[x][y] = 41;
			a[x][y].init();
		}
	}
	for (x=0;x<DIMX;x++)
	{
		t[x][0] = 26;
		t[x][DIMY-1] = 26;
	}
	for (y=0;y<DIMY;y++)
	{
		t[0][y] = 17;
		t[DIMX-1][y] = 17;
	}
	t[0][0] = 19;
	t[DIMX-1][0] = 18;
	t[0][DIMY-1] = 29;
	t[DIMX-1][DIMY-1] = 14;
	t[1][1] = 283;
	posx = 1;
	posy = 1;
}

void Level::Load (FILE *f)
{
	printf("Level::Load File\n");
	//initialize the directions
	dir = 1;
	tdir = 1;
	//initialize the object
	object = 0;
	//initialize the animation array
	int i,j;
	for (j=0;j<DIMY;j++)
	{
		for (i=0;i<DIMX;i++)
		{
			a[i][j].init();
		}
	}
	int x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (!Read(&t[x][y],f))
			{
				fprintf(stderr,"Read error !!!\n");
				return;
			}
			if (t[x][y] == 11)
			{
				posx = x;
				posy = y;
				t[x][y] = 283; //respawn point
			}
			if (t[x][y] >= NI)
				t[x][y] = 0;
		}
	}
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (!fread(&ground[x][y],sizeof(Uint8),1,f))
			{
				fprintf(stderr,"Error while reading the ground !!!\n");
				return;
			}
		}
	}
	//respawn at the right place
	if (sx != 0 || sy != 0)
	{
		posx = sx;
		posy = sy;
		object = so;
	}
}

void Level::Save (FILE *f)
{
	printf("Level::Save File\n");
	int x,y;
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (!Write(t[x][y],f))
			{
				fprintf(stderr,"Write error !!!\n");
				return;
			}
		}
	}
	for (y=0;y<DIMY;y++)
	{
		for (x=0;x<DIMX;x++)
		{
			if (!fwrite(&ground[x][y],sizeof(Uint8),1,f))
			{
				fprintf(stderr,"Error while writing the ground !!!\n");
				return;
			}
		}
	}
}

void Level::Load (Uint8 n) //loads level number n
{
	printf("Level::Load Uint8 %d\n", n);
	char buf[256];
#if EDIT
	char txt[40];
	sprintf(txt,"level%d",n);
	Load(txt);
#else
	MakeName("LEVEL.DAT", buf, RESOURCES, 256);
	FILE *f = fopen(buf,"r");
	if (!f)
	{
		fprintf(stderr,"Unable to open the level file !\n");
		return;
	}
	Uint16 N; //number of levels
	Read(&N,f);
	if (n >= N)
	{
		printf("Level %d doesn't exist (levels 0 to %d) !!!\n",n,N-1);
		return;
	}
	if (fseek(f,40*36*2*n,SEEK_CUR))
	{
		printf("Error : fseek failed !!!\n");
	}
	Load(f);
	fclose(f);
#endif
}

void Level::Save (Uint8 n) //saves level number n
{
	printf("Level::Save Uint8 %d\n", n);
	char buf[256];
#if EDIT
	char txt[40];
	sprintf(txt,"level%d",n);
	Save(txt);
#else
	MakeName("LEVEL.DAT", buf, RESOURCES, 256);
	FILE *f = fopen(buf,"r+");
	if (!f)
	{
		fprintf(stderr,"Unable to open the level file !\n");
		return;
	}
	Uint16 N; //number of levels
	Read(&N,f);
	if (n >= N)
	{
		printf("Level %d doesn't exist (levels 0 to %d) !!!\n",n,N-1);
		return;
	}
	if (fseek(f,40*36*2*n,SEEK_CUR))
	{
		printf("Error : fseek failed !!!\n");
	}
	Save(f);
	fclose(f);
#endif
}

void Level::Load (const char *file)
{
	printf("Level::Load constChar File\n");
	Default();
	char buf[256];
	MakeName(file, buf, LEVELS_DIRECTORY, 256);
	FILE *f = fopen(buf,"r");
	if (!f)
	{
		fprintf(stderr,"Unable to open file %s for reading !\n",buf);
		return;
	}
	Load(f);
	fclose(f);
}

void Level::Save (const char *file)
{
	printf("Level::Save constChar File\n");
	char buf[256];
	MakeName(file, buf, LEVELS_DIRECTORY, 256);
	FILE *f = fopen(buf,"w");
	if (!f)
	{
		fprintf(stderr,"Unable to open file %s for writing !\n",buf);
		return;
	}
	Save(f);
	fclose(f);
}

