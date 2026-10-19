bool SOUND = true;

#define NSOUNDSM	26 //max number of sounds to load in memory
#define NSOUNDS 32 //number of sounds that can be simultaneously played

typedef struct {
	Uint8 *data;
	Uint32 size;
}Sound;


Sound LoadSound (const char *file)
{
	SDL_AudioSpec wav_spec;
	Uint32 wav_length;
	Uint8 *wav_buff;
	SDL_AudioCVT  wav_cvt;
	
	SDL_AudioSpec desired;
	desired.freq = 11050;
	desired.format = AUDIO_U8;
	//desired.samples = 256;
	//desired.callback = NULL;
	desired.channels = 1;
	//desired.userdata = NULL;
	
	if(SDL_LoadWAV(file, &wav_spec, &wav_buff, &wav_length) == NULL)
	{
		fprintf(stderr, "Unable to open %s : %s\n", file, SDL_GetError());
		exit(EXIT_FAILURE);
	}
	
	Sound s;
	
	if (wav_spec.freq == 5525)
	{
		//conversion
		Uint8 *data = (Uint8 *)malloc(sizeof(Uint8)*wav_length*2);
		if (!data)
		{
			fprintf(stderr,"Out of memory !!!\n");
		}
		Uint32 i;
		Uint8 *from = (Uint8 *)wav_buff;
		Uint8 *dest = data;
		Uint8 rf =0;
		for (i=0;i<wav_length;i++)
		{
			*dest = (*from+2*rf)/3;
			dest++;
			*dest = (*from*2+rf)/3;
			rf = *from;
			dest++;
			from++;
		}
		SDL_FreeWAV(wav_buff);
		wav_buff = (Uint8 *)data;
		wav_length *= 2;
	}
	 
	/*
	Uint8 *data = (Uint8 *)malloc(wav_length);
	memcpy(data, wav_buff, wav_length);
	// We can delete to original WAV data now
	SDL_FreeWAV(wav_buff);
	wav_buff = data;
	*/
	
	s.data = wav_buff; //wav_cvt.buf;
	
	if (!s.data)
	{
		fprintf(stderr,"Error : No sound after conversion !!!\n");
	}
	
	s.size = wav_length; //wav_cvt.len_cvt;
	//printf("Size of sound : before %d, after %d\n",wav_cvt.len,s.size);
	//printf("Size of sound : %d\n",wav_length);
	return s;
}

typedef struct 
{
	Uint8 *ptr; //current play position (NULL if the sound is not playing)
	Uint32 i; //sound index
}RefSound;

void mixaudio(void *unused, Uint8 *stream, int len);

struct Sounds
{
	Sound s[NSOUNDSM];
	RefSound r[NSOUNDS];
	
	//
	void Init (); //initializes everything (and load sounds)
	void Load (); //load all sounds
	void Load (const char *file, Uint16 i); //Load the i-th sound (needs to be .wav)
	void init (); //initializes the structure
	void Start (); //starts the sound system
	void Stop (); //stops the sound system
	void Play (Uint16 i); //plays the i-th sound
};

void Sounds::Init ()
{
	init(); //
	Start(); //
	Load(); //
}
void Sounds::Load (const char *file, Uint16 i)
{
	s[i] = LoadSound(file);
}
void Sounds::Load ()
{
	int i;
	char file[256];
	char buf[256];
	MakeName("son00.wav", file, SOUNDS_DIRECTORY, 256);
	for (i=1;i<NSOUNDSM;i++)
	{
		file[strlen(SOUNDS_DIRECTORY)+3] = '0'+i/10;
		file[strlen(SOUNDS_DIRECTORY)+4] = '0'+i%10;
		MakeName(file, buf, RESOURCES, 256);
		Load(buf, i);
	}
}
void Sounds::init ()
{
	int i;
	for (i=0;i<NSOUNDSM;i++)
	{
		s[i].data = NULL;
		s[i].size = 0;
	}
	for (i=0;i<NSOUNDS;i++)
	{
		r[i].ptr = NULL;
		r[i].i = 0;
	}
}
void Sounds::Start ()
{
	init();
	SDL_AudioSpec desired,obtained;
	desired.freq = 11050;
	desired.format = AUDIO_U8;
	desired.samples = 256;
	desired.callback = mixaudio;
	desired.channels = 1;
	desired.userdata = this;
	if (SDL_OpenAudio(&desired,&obtained) < 0)
	{
		printf("Unable to open Audio : %s\n",SDL_GetError());
		exit(EXIT_FAILURE);
	}
	if (obtained.freq != desired.freq)
	{
		printf("Frequency not reached !!!!\n");
	}
	/*
	printf("Audio obtained:\n");
	printf("freq : %d\n",obtained.freq);
	printf("samples : %d\n",obtained.samples);
	printf("channels : %d\n",obtained.channels);
	printf("format : %d\n",obtained.format);
	
	printf("List of formats:\n");
	printf("AUDIO_U8 = %d\n",AUDIO_U8);
	printf("AUDIO_S8 = %d\n",AUDIO_S8);
	printf("AUDIO_U16 = %d\n",AUDIO_U16);
	printf("AUDIO_S16 = %d\n",AUDIO_S16);
	printf("AUDIO_U16MSB = %d\n",AUDIO_U16MSB);
	printf("AUDIO_S16MSB = %d\n",AUDIO_S16MSB);
	printf("AUDIO_U16SYS = %d\n",AUDIO_U16SYS);
	printf("AUDIO_S16SYS = %d\n",AUDIO_S16SYS);
	*/
	//start the sound
	SDL_PauseAudio(0);
}
void Sounds::Stop ()
{
	 SDL_CloseAudio();
}
void Sounds::Play (Uint16 i) //plays the i-th sound
{
#if HSOUND
	if (!SOUND)
		return;
#else
	return;
#endif
	if (i == 0)
		return;
	if (i >= NSOUNDSM)
	{
		fprintf(stderr, "Sound file doesn't exist !!!\n");
		return;
	}
	//looking for a free slot
	///SDL_LockAudio();
	int j;
	for (j=0;j<NSOUNDS;j++)
	{
		if (r[j].ptr == NULL)
		{ //starts sounds
			//printf("Launch of %d in %d ...\n",i,j);
			r[j].ptr = s[i].data;
			r[j].i = i;
			break;
		}
	}
	///SDL_UnlockAudio();
}

void mixaudio(void *userdata, Uint8 *stream, int len)
{
	int i;
	Uint32 amount;
	Sounds *s = (Sounds *)userdata;
	for (i=0;i<NSOUNDS;i++)
	{
		if (s->r[i].ptr != NULL)
		{
			//printf("sound %d...\n",i);
			amount = s->s[s->r[i].i].size - (s->r[i].ptr - s->s[s->r[i].i].data); //amount of remaining to play
			if ( amount > len )
			{
				amount = len;
			}
			SDL_MixAudio(stream, s->r[i].ptr, amount, SDL_MIX_MAXVOLUME);
			s->r[i].ptr += amount;
			if (amount < len)
			{
				s->r[i].ptr = NULL;
			}
		}
	}
}
