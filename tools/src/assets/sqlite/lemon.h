#pragma once

struct lemon;
struct symbol;

void Symbol_init(void);
struct symbol *Symbol_new(const char *);
int Symbol_count(void);
struct symbol **Symbol_arrayof(void);

void Parse(struct lemon *lemp);
void ReportOutput(struct lemon *lemp);

typedef enum {LEMON_FALSE=0, LEMON_TRUE} Boolean;

enum symbol_type {
    TERMINAL,
    NONTERMINAL,
    MULTITERMINAL
};
enum e_assoc {
      LEFT,
      RIGHT,
      NONE,
      UNK
};
struct symbol {
    const char *name;        /* Name of the symbol */
    int index;               /* Index number for this symbol */
    enum symbol_type type;   /* Symbols are all either TERMINALS or NTs */
    struct rule *rule;       /* Linked list of rules of this (if an NT) */
    struct symbol *fallback; /* fallback token in case this token doesn't parse */
    int prec;                /* Precedence if defined (-1 otherwise) */
    enum e_assoc assoc;      /* Associativity if precedence is defined */
    char *firstset;          /* First-set for all rules of this symbol */
    Boolean lambda;          /* True if NT and can generate an empty string */
    int useCnt;              /* Number of times used */
    char *destructor;        /* Code which executes whenever this symbol is
                             ** popped from the stack during error processing */
    int destLineno;          /* Line number for start of destructor.  Set to
                             ** -1 for duplicate destructors. */
    char *datatype;          /* The data type of information held by this
                             ** object. Only used if type==NONTERMINAL */
    int dtnum;               /* The data type number.  In the parser, the value
                             ** stack is a union.  The .yy%d element of this
                             ** union is the correct data type for this object */
    int bContent;            /* True if this symbol ever carries content - if
                             ** it is ever more than just syntax */
    /* The following fields are used by MULTITERMINALs only */
    int nsubsym;             /* Number of constituent symbols in the MULTI */
    struct symbol **subsym;  /* Array of constituent symbols */
};
  
/* Each production rule in the grammar is stored in the following
** structure.  */
struct rule {
    struct symbol *lhs;      /* Left-hand side of the rule */
    const char *lhsalias;    /* Alias for the LHS (NULL if none) */
    int lhsStart;            /* True if left-hand side is the start symbol */
    int ruleline;            /* Line number for the rule */
    int nrhs;                /* Number of RHS symbols */
    struct symbol **rhs;     /* The RHS symbols */
    const char **rhsalias;   /* An alias for each RHS symbol (NULL if none) */
    int line;                /* Line number at which code begins */
    const char *code;        /* The code executed when this rule is reduced */
    const char *codePrefix;  /* Setup code before code[] above */
    const char *codeSuffix;  /* Breakdown code after code[] above */
    struct symbol *precsym;  /* Precedence symbol for this rule */
    int index;               /* An index number for this rule */
    int iRule;               /* Rule number as used in the generated tables */
    Boolean noCode;          /* True if this rule has no associated C code */
    Boolean codeEmitted;     /* True if the code has been emitted already */
    Boolean canReduce;       /* True if this rule is ever reduced */
    Boolean doesReduce;      /* Reduce actions occur after optimization */
    Boolean neverReduce;     /* Reduce is theoretically possible, but prevented
                             ** by actions or other outside implementation */
    struct rule *nextlhs;    /* Next rule with the same LHS */
    struct rule *next;       /* Next rule in the global list */
};
  
/* A configuration is a production rule of the grammar together with
** a mark (dot) showing how much of that rule has been processed so far.
** Configurations also contain a follow-set which is a list of terminal
** symbols which are allowed to immediately follow the end of the rule.
** Every configuration is recorded as an instance of the following: */
enum cfgstatus {
    COMPLETE,
    INCOMPLETE
};
struct config {
    struct rule *rp;         /* The rule upon which the configuration is based */
    int dot;                 /* The parse point */
    char *fws;               /* Follow-set for this configuration only */
    struct plink *fplp;      /* Follow-set forward propagation links */
    struct plink *bplp;      /* Follow-set backwards propagation links */
    struct state *stp;       /* Pointer to state which contains this */
    enum cfgstatus status;   /* used during followset and shift computations */
    struct config *next;     /* Next configuration in the state */
    struct config *bp;       /* The next basis configuration */
};
  
enum e_action {
    SHIFT,
    ACCEPT,
    REDUCE,
    ERROR,
    SSCONFLICT,              /* A shift/shift conflict */
    SRCONFLICT,              /* Was a reduce, but part of a conflict */
    RRCONFLICT,              /* Was a reduce, but part of a conflict */
    SH_RESOLVED,             /* Was a shift.  Precedence resolved conflict */
    RD_RESOLVED,             /* Was reduce.  Precedence resolved conflict */
    NOT_USED,                /* Deleted by compression */
    SHIFTREDUCE              /* Shift first, then reduce */
};
  
  /* Every shift or reduce operation is stored as one of the following */
  struct action {
    struct symbol *sp;       /* The look-ahead symbol */
    enum e_action type;
    union {
      struct state *stp;     /* The new state, if a shift */
      struct rule *rp;       /* The rule, if a reduce */
    } x;
    struct symbol *spOpt;    /* SHIFTREDUCE optimization to this symbol */
    struct action *next;     /* Next action for this state */
    struct action *collide;  /* Next action with the same hash */
};

/* Each state of the generated parser's finite state machine
** is encoded as an instance of the following structure. */
struct state {
    struct config *bp;       /* The basis configurations for this state */
    struct config *cfp;      /* All configurations in this set */
    int statenum;            /* Sequential number for this state */
    struct action *ap;       /* List of actions for this state */
    int nTknAct, nNtAct;     /* Number of actions on terminals and nonterminals */
    int iTknOfst, iNtOfst;   /* yy_action[] offset for terminals and nonterms */
    int iDfltReduce;         /* Default action is to REDUCE by this rule */
    struct rule *pDfltReduce;/* The default REDUCE rule. */
    int autoReduce;          /* True if this is an auto-reduce state */
};
#define NO_OFFSET (-2147483647)
  
/* The state vector for the entire parser generator is recorded as
** follows.  (LEMON uses no global variables and makes little use of
** static variables.  Fields in the following structure can be thought
** of as begin global variables in the program.) */
struct lemon {
    struct state **sorted;   /* Table of states sorted by state number */
    struct rule *rule;       /* List of all rules */
    struct rule *startRule;  /* First rule */
    int nstate;              /* Number of states */
    int nxstate;             /* nstate with tail degenerate states removed */
    int nrule;               /* Number of rules */
    int nruleWithAction;     /* Number of rules with actions */
    int nsymbol;             /* Number of terminal and nonterminal symbols */
    int nterminal;           /* Number of terminal symbols */
    int minShiftReduce;      /* Minimum shift-reduce action value */
    int errAction;           /* Error action value */
    int accAction;           /* Accept action value */
    int noAction;            /* No-op action value */
    int minReduce;           /* Minimum reduce action */
    int maxAction;           /* Maximum action value of any kind */
    struct symbol **symbols; /* Sorted array of pointers to symbols */
    int errorcnt;            /* Number of errors */
    struct symbol *errsym;   /* The error symbol */
    struct symbol *wildcard; /* Token that matches anything */
    char *name;              /* Name of the generated parser */
    char *arg;               /* Declaration of the 3rd argument to parser */
    char *ctx;               /* Declaration of 2nd argument to constructor */
    char *tokentype;         /* Type of terminal symbols in the parser stack */
    char *vartype;           /* The default type of non-terminal symbols */
    char *start;             /* Name of the start symbol for the grammar */
    char *stacksize;         /* Size of the parser stack */
    char *include;           /* Code to put at the start of the C file */
    char *error;             /* Code to execute when an error is seen */
    char *overflow;          /* Code to execute on a stack overflow */
    char *failure;           /* Code to execute on parser failure */
    char *accept;            /* Code to execute when the parser excepts */
    char *extracode;         /* Code appended to the generated file */
    char *tokendest;         /* Code to execute to destroy token data */
    char *vardest;           /* Code for the default non-terminal destructor */
    char *filename;          /* Name of the input file */
    char *outname;           /* Name of the current output file */
    char *tokenprefix;       /* A prefix added to token names in the .h file */
    char *reallocFunc;       /* Function to use to allocate stack space */
    char *freeFunc;          /* Function to use to free stack space */
    int nconflict;           /* Number of parsing conflicts */
    int nactiontab;          /* Number of entries in the yy_action[] table */
    int nlookaheadtab;       /* Number of entries in yy_lookahead[] */
    int tablesize;           /* Total table size of all tables in bytes */
    int basisflag;           /* Print only basis configurations */
    int printPreprocessed;   /* Show preprocessor output on stdout */
    int has_fallback;        /* True if any %fallback is seen in the grammar */
    int nolinenosflag;       /* True if #line statements should not be printed */
    int argc;                /* Number of command-line arguments */
    char **argv;             /* Command-line arguments */
};


/* There is one instance of the following structure for each
** associative array of type "x2".
*/
struct s_x2 {
    int size;               /* The number of available slots. */
                            /*   Must be a power of 2 greater than or */
                            /*   equal to 1 */
    int count;              /* Number of currently slots filled */
    struct s_x2node *tbl;  /* The data stored here */
    struct s_x2node **ht;  /* Hash table for lookups */
  };
  
  /* There is one instance of this structure for every data element
  ** in an associative array of type "x2".
  */
  typedef struct s_x2node {
    struct symbol *data;     /* The data */
    const char *key;         /* The key */
    struct s_x2node *next;   /* Next entry with the same hash */
    struct s_x2node **from;  /* Previous link */
  } x2node;
  
  /* There is only one instance of the array, which is the following */
  static struct s_x2 *x2a;
  
