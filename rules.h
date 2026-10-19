
//Masks for the types
enum
{
	wall = 1<<0,
	object = 1<<1,
	enemy = 1<<2,
	explosive = 1<<3,
	react_ball = 1<<4,
	react_on = 1<<5,
	missile = 1<<6,
	stop_ball = 1<<7,
	door = 1<<8, //one-way door
	door_open = 1<<9, //open one-way door
	onda_mania = 1<<10,
	rail = 1<<11,
	rail_active = 1<<12,
	plant = 1<<13,
	colored_circle = 1<<14,
	teleporter = 1<<15,
	rail_wall = 1<<16,
	act_ball = 1<<17,
	conveyor_belt = 1<<18,
	arrow = 1<<19,
	computer_thing = 1<<20,
	onda_mania_moving = 1<<21,
	ground = 1<<22,
	door_ = 1<<23,
	rail_turning = 1<<24,
	wellable = 1<<25,
	real_wall = 1<<26,
	real_plant = 1<<27,
	ground_arrow = 1<<28
};

struct Rules
{
	Uint32 type; //type of object
	
	void init ();
	inline bool is (Uint32 type2);
};

inline bool Rules::is (Uint32 type2)
{
	return type & type2;
}

void Rules::init ()
{
	type = 0;
}

Uint32 getdefnum (const char *word)
{
	if (!strcmp(word, "wall"))
		return wall;
	if (!strcmp(word, "object"))
		return object;
	if (!strcmp(word, "enemy"))
		return enemy;
	if (!strcmp(word, "explosive"))
		return explosive;
	if (!strcmp(word, "react_ball"))
		return react_ball;
	if (!strcmp(word, "react_on"))
		return react_on;
	if (!strcmp(word, "missile"))
		return missile;
	if (!strcmp(word, "stop_ball"))
		return stop_ball;
	if (!strcmp(word, "door"))
		return door;
	if (!strcmp(word, "door_open"))
		return door_open;
	if (!strcmp(word, "onda-mania"))
		return onda_mania;
	if (!strcmp(word, "rail"))
		return rail;
	if (!strcmp(word, "rail_active"))
		return rail_active;
	if (!strcmp(word, "plant"))
		return plant;
	if (!strcmp(word, "colored_circle"))
		return colored_circle;
	if (!strcmp(word, "teleporter"))
		return teleporter;
	if (!strcmp(word, "rail_wall"))
		return rail_wall;
	if (!strcmp(word, "act_ball"))
		return act_ball;
	if (!strcmp(word, "conveyor_belt"))
		return conveyor_belt;
	if (!strcmp(word, "arrow"))
		return arrow;
	if (!strcmp(word, "computer_thing"))
		return computer_thing;
	if (!strcmp(word, "onda_mania_moving"))
		return onda_mania_moving;
	if (!strcmp(word, "ground"))
		return ground;
	if (!strcmp(word, "door_"))
		return door_;
	if (!strcmp(word, "rail_turning"))
		return rail_turning;
	if (!strcmp(word, "wellable"))
		return wellable;
	if (!strcmp(word, "real_wall"))
		return real_wall;
	if (!strcmp(word, "real_plant"))
		return real_plant;
	if (!strcmp(word, "ground_arrow"))
		return ground_arrow;
	return 0;
}

void LoadRules (Rules *r)
{
	printf("Load Rules");
	int i;
	for (i=0;i<NI;i++)
	{
		r[i].init();
	}
	char buf[256];
	MakeName("rules.txt", buf, RESOURCES, 256);
	FILE *f = fopen(buf,"r");
	if (!f)
	{
		fprintf(stderr, "Unable to load rules !!!\n");
		return;
	}
	
	char word[256];
	const char *ptr;
	Uint32 def = 0;
	Uint16 n;
		
	for (i=0;;i++)
	{
		//printf("Reading line %d : ",i);
		if (!readline(f, buf, 256))
			break;
		//printf(buf);
		//printf("\n");
		ptr = buf;
		if (ptr = findword(buf, word, 256)) //looking if new defintion
		{ //beginning of a new definition
			//printf(" word is : %s , remaining : %s\n",word,ptr);
			if (!(def = getdefnum (word)))
			{
				fprintf(stderr, "Type %s is unknown !\n",word);
			}
		}else
			ptr = buf;
		if (def)
		{
			for(;;) //seeks all definitions
			{
				ptr = findnum (ptr, &n);
				if (!ptr)
					break;
				//printf("number is : %d\n",n);
				//This new feature adds the object n
				if (n < NI)
					r[n].type |= def;
				else
				{
					fprintf(stderr, "Error : Definition of the object %d doesn't exist !!!\n",n);
				}
			}
		}
	}
	
	fclose(f);
}
