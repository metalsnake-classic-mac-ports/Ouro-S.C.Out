void MakeName (const char *txt, char *res, const char *path, Uint16 bufLength)
{
	strlcpy(res, path, bufLength);
	strlcat(res, txt, bufLength);
}



bool Read (Uint16 *n, FILE *f)
{
	Uint8 c;
	Uint16 N;
	if (!fread(&c,sizeof(c),1,f))
		return false;
	N = c;
	if (!fread(&c,sizeof(c),1,f))
		return false;
	N |= c<<8;
	*n = N;
	return true;
}
bool Write (Uint16 n, FILE *f)
{
	Uint8 c;
	c = n%256;
	if (!fwrite(&c,sizeof(c),1,f))
		return false;
	c = n/256;
	if (!fwrite(&c,sizeof(c),1,f))
		return false;
	return true;
}
/*
bool Read (Uint16 *n, int f)//FILE *f)
{
	Uint8 c;
	Uint16 N;
	//if (!fread(&c,sizeof(c),1,f))
	//      return false;
	read(f, &c, 1);
	//return false;
	N = c;
	//if (!fread(&c,sizeof(c),1,f))
	//      return false;
	read(f, &c, 1);
	N |= c<<8;
	*n = N;
	return true;
}
*/
int seek (int f, Uint32 pos)
{
	Uint32 i;
	unsigned char c;
	for (i=0;i<pos;i++)
	{
			read(f, &c, 1);
	}
	return 0;
}

bool readline (FILE *f, char *txt, long max)
{
	char c,rc;
	c = 0;
	int i;
	for (i=0;;i++)
	{
		rc = c;
		if (!fread(&c,sizeof(c),1,f) || i>=max)
		{
			if (i>=max)
				printf("Warning : line too long !!!\n");
			return false;
		}
		if (c=='/' && rc=='/')
		{
			txt--;
			*txt = 0;
			txt++;
		}
		if (c == '\n' || c == 13 || c == 0)
		{
			*txt = 0;
			break;
		}
		*txt = c;
		txt++;
	}
	return true;
}

const char *findword (const char *buf, char *word, int max) //looks for a variable name
{
	int i;
	bool started = false;
	for (i=0;i<max;i++)
	{
		if ((*buf >= 'a' && *buf <= 'z') || (*buf >= 'A' && *buf <= 'Z'))
			started = true;
		if (started)
		{
			if (*buf == ' ' || *buf == '	' || *buf == 0 || *buf == '\n' || *buf == 13)
			{
				*word = 0;
				return buf;
			}
			*word = *buf;
			word++;
		}else
		{
			if (*buf == 0 || *buf == '\n' || *buf == 13)
				return NULL;
		}
		buf++;
	}
	return NULL;
}

const char *pass (const char *txt) //skips spaces
{
	while(*txt == ' ' || *txt == '	' || *txt == '\240')
	{
		txt++;
	}
	return txt;
}

const char *readnum (const char *txt, int *n) //reads a positive integer
{
	if (*txt < '0' || *txt > '9')
		return NULL;
	*n = 0;
	while (*txt >= '0' && *txt <= '9')
	{
		*n = *n * 10;
		*n = *n + (*txt-'0');
		txt++;
	}
	return txt;
}

const char *findstart (const char *txt, int *n, int max)
{
	int i = 0;
	int n2;
	//skips spaces
	txt = pass(txt);
	//reads an integer
	txt = readnum(txt, &n2);
	if (!txt)
		return NULL;
	//skips spaces
	txt = pass(txt);
	//reads ':'
	if (*txt == ':')
	{
		*n = n2;
		return txt+1;
	}else
		return NULL;
}

const char *findnum (const char *word, Uint16 *n)
{
	/*
	bool started = false;
	int i = 0;
	*n = 0;
	for (;;)
	{
		if (*word >= '0' && *word <= '9')
			started = true;
		if (started)
		{
			if (*word < '0' || *word > '9')
			{
				return word;
			}
			*n = *n * 10;
			*n += *word - '0';
			i++;
			if (i >= 6)
				return NULL;
		}else
		{
			if (*word != ' ' && *word != '	')
				return NULL;
		}
		word++;
	}
	*/
	//skips spaces
	word = pass(word);
	//reads a number
	int n2;
	word = readnum(word,&n2);
	*n = n2;
	return word;
}
