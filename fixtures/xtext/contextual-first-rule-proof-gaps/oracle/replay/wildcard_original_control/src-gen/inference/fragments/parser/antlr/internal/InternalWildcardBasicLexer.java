package inference.fragments.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalWildcardBasicLexer extends Lexer {
    public static final int RULE_ID=4;
    public static final int T__6=6;
    public static final int RULE_WS=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalWildcardBasicLexer() {;} 
    public InternalWildcardBasicLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalWildcardBasicLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalWildcardBasic.g"; }

    // $ANTLR start "T__6"
    public final void mT__6() throws RecognitionException {
        try {
            int _type = T__6;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalWildcardBasic.g:11:6: ( 'm' )
            // InternalWildcardBasic.g:11:8: 'm'
            {
            match('m'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__6"

    // $ANTLR start "RULE_ID"
    public final void mRULE_ID() throws RecognitionException {
        try {
            int _type = RULE_ID;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalWildcardBasic.g:126:9: ( ( 'a' .. 'z' )+ )
            // InternalWildcardBasic.g:126:11: ( 'a' .. 'z' )+
            {
            // InternalWildcardBasic.g:126:11: ( 'a' .. 'z' )+
            int cnt1=0;
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( ((LA1_0>='a' && LA1_0<='z')) ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalWildcardBasic.g:126:12: 'a' .. 'z'
            	    {
            	    matchRange('a','z'); 

            	    }
            	    break;

            	default :
            	    if ( cnt1 >= 1 ) break loop1;
                        EarlyExitException eee =
                            new EarlyExitException(1, input);
                        throw eee;
                }
                cnt1++;
            } while (true);


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_ID"

    // $ANTLR start "RULE_WS"
    public final void mRULE_WS() throws RecognitionException {
        try {
            int _type = RULE_WS;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalWildcardBasic.g:128:9: ( ( ' ' | '\\t' | '\\r' | '\\n' )+ )
            // InternalWildcardBasic.g:128:11: ( ' ' | '\\t' | '\\r' | '\\n' )+
            {
            // InternalWildcardBasic.g:128:11: ( ' ' | '\\t' | '\\r' | '\\n' )+
            int cnt2=0;
            loop2:
            do {
                int alt2=2;
                int LA2_0 = input.LA(1);

                if ( ((LA2_0>='\t' && LA2_0<='\n')||LA2_0=='\r'||LA2_0==' ') ) {
                    alt2=1;
                }


                switch (alt2) {
            	case 1 :
            	    // InternalWildcardBasic.g:
            	    {
            	    if ( (input.LA(1)>='\t' && input.LA(1)<='\n')||input.LA(1)=='\r'||input.LA(1)==' ' ) {
            	        input.consume();

            	    }
            	    else {
            	        MismatchedSetException mse = new MismatchedSetException(null,input);
            	        recover(mse);
            	        throw mse;}


            	    }
            	    break;

            	default :
            	    if ( cnt2 >= 1 ) break loop2;
                        EarlyExitException eee =
                            new EarlyExitException(2, input);
                        throw eee;
                }
                cnt2++;
            } while (true);


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_WS"

    public void mTokens() throws RecognitionException {
        // InternalWildcardBasic.g:1:8: ( T__6 | RULE_ID | RULE_WS )
        int alt3=3;
        switch ( input.LA(1) ) {
        case 'm':
            {
            int LA3_1 = input.LA(2);

            if ( ((LA3_1>='a' && LA3_1<='z')) ) {
                alt3=2;
            }
            else {
                alt3=1;}
            }
            break;
        case 'a':
        case 'b':
        case 'c':
        case 'd':
        case 'e':
        case 'f':
        case 'g':
        case 'h':
        case 'i':
        case 'j':
        case 'k':
        case 'l':
        case 'n':
        case 'o':
        case 'p':
        case 'q':
        case 'r':
        case 's':
        case 't':
        case 'u':
        case 'v':
        case 'w':
        case 'x':
        case 'y':
        case 'z':
            {
            alt3=2;
            }
            break;
        case '\t':
        case '\n':
        case '\r':
        case ' ':
            {
            alt3=3;
            }
            break;
        default:
            NoViableAltException nvae =
                new NoViableAltException("", 3, 0, input);

            throw nvae;
        }

        switch (alt3) {
            case 1 :
                // InternalWildcardBasic.g:1:10: T__6
                {
                mT__6(); 

                }
                break;
            case 2 :
                // InternalWildcardBasic.g:1:15: RULE_ID
                {
                mRULE_ID(); 

                }
                break;
            case 3 :
                // InternalWildcardBasic.g:1:23: RULE_WS
                {
                mRULE_WS(); 

                }
                break;

        }

    }


 

}