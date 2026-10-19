
#define NA	50	//max number of animations

#define NIM	20 //max number of frames in an animation

struct Anim
{
	Uint16 l[NIM];	//list of images
	Uint16 n;	//number of frames
	Uint32	t; //time between two images
	Uint16	s; //sound
	
	void init ();
};

void Anim::init ()
{
	n = 0;
	t = 0;
	s = 0;
}

void LoadAnims (Anim *a)
{
	//printf("\n\n\n\n");	
	int i;
	for (i=0;i<NA;i++)
	{
		a[i].init();
	}
	char buf[256];
	MakeName("anims.txt", buf, RESOURCES, 256);
	FILE *f = fopen(buf,"r");
	if (!f)
	{
		fprintf(stderr, "Unable to load animations !!!\n");
		return;
	}
	
	const char *ptr, *ptr2;
	int n = -1; //animation number
	Uint16 k; //image number
		
	for (i=0;;i++)
	{
		//printf("Reading line %d : ",i);
		if (!readline(f, buf, 256))
		{
			//printf("end...\n");
			break;
		}
		//printf(buf);
		//printf("\n");
		ptr = findstart(buf, &n, 256); //look if there is the beginning of a new animation
		if (ptr == NULL)
			ptr = buf;
		else
		{
			//printf("Animation %d ...\n",n);
		}
		if (n >= NA)
		{
			fprintf(stderr, "Error : animation index too big (%d) !!!\n",n);
			return;
		}
		if (n != -1)
		{
			for(;;) //seeks all animation images 
			{
				ptr2 = findnum (ptr, &k);
				if (!ptr2)
				{
					break;
				}
				ptr = ptr2;
				//printf("number is : %d\n",k);
				//adds the image later
				if (k < NI)
				{
					if (a[n].n < NIM)
					{
						a[n].l[a[n].n] = k;
						a[n].n++;
					}else
						fprintf(stderr, "Error : too many images for animation %d !\n",n);
				}else
				{
					fprintf(stderr, "Error : Image %d of animation %d (%d) doesn't exist !!!\n",a[n].n,n,k);
				}
			}
			//skip spaces
			ptr = pass(ptr);
			if (*ptr == ':')
			{
				ptr++;
				//wait time between two images
				Uint16 t2;
				a[n].t = 1; //timer default
				ptr2 = findnum(ptr, &t2);
				if (ptr2)
				{
					ptr = ptr2;
					a[n].t = t2;
					//printf("time between two images : %d\n",t2);
				}
			}
			//skip spaces
			ptr = pass(ptr);
			if (*ptr == ':')
			{
				ptr++;
				//reads the sound number
				Uint16 s2;
				ptr2 = findnum(ptr, &s2);
				if (ptr2)
				{
					ptr = ptr2;
					a[n].s = s2;
					//printf("sound : %d\n",s2);
				}
			}
		}
	}
	
	fclose(f);
}
