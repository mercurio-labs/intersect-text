package audit.scopes.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalCallerSuffixLexer extends Lexer {
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=5;
    public static final int RULE_WS=4;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int T__12=12;
    public static final int T__13=13;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators

    public InternalCallerSuffixLexer() {;} 
    public InternalCallerSuffixLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalCallerSuffixLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalCallerSuffix.g"; }

    // $ANTLR start "T__8"
    public final void mT__8() throws RecognitionException {
        try {
            int _type = T__8;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:11:6: ( '!' )
            // InternalCallerSuffix.g:11:8: '!'
            {
            match('!'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__8"

    // $ANTLR start "T__9"
    public final void mT__9() throws RecognitionException {
        try {
            int _type = T__9;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:12:6: ( '?' )
            // InternalCallerSuffix.g:12:8: '?'
            {
            match('?'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__9"

    // $ANTLR start "T__10"
    public final void mT__10() throws RecognitionException {
        try {
            int _type = T__10;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:13:7: ( 'direct' )
            // InternalCallerSuffix.g:13:9: 'direct'
            {
            match("direct"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__10"

    // $ANTLR start "T__11"
    public final void mT__11() throws RecognitionException {
        try {
            int _type = T__11;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:14:7: ( 'done' )
            // InternalCallerSuffix.g:14:9: 'done'
            {
            match("done"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__11"

    // $ANTLR start "T__12"
    public final void mT__12() throws RecognitionException {
        try {
            int _type = T__12;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:15:7: ( 'a' )
            // InternalCallerSuffix.g:15:9: 'a'
            {
            match('a'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__12"

    // $ANTLR start "T__13"
    public final void mT__13() throws RecognitionException {
        try {
            int _type = T__13;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:16:7: ( 'b' )
            // InternalCallerSuffix.g:16:9: 'b'
            {
            match('b'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__13"

    // $ANTLR start "RULE_ID"
    public final void mRULE_ID() throws RecognitionException {
        try {
            int _type = RULE_ID;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:286:9: ( ( 'a' .. 'z' | '\\u00E9' )+ )
            // InternalCallerSuffix.g:286:11: ( 'a' .. 'z' | '\\u00E9' )+
            {
            // InternalCallerSuffix.g:286:11: ( 'a' .. 'z' | '\\u00E9' )+
            int cnt1=0;
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( ((LA1_0>='a' && LA1_0<='z')||LA1_0=='\u00E9') ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalCallerSuffix.g:
            	    {
            	    if ( (input.LA(1)>='a' && input.LA(1)<='z')||input.LA(1)=='\u00E9' ) {
            	        input.consume();

            	    }
            	    else {
            	        MismatchedSetException mse = new MismatchedSetException(null,input);
            	        recover(mse);
            	        throw mse;}


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
            // InternalCallerSuffix.g:288:9: ( ( ' ' | '\\t' | '\\r' | '\\n' )+ )
            // InternalCallerSuffix.g:288:11: ( ' ' | '\\t' | '\\r' | '\\n' )+
            {
            // InternalCallerSuffix.g:288:11: ( ' ' | '\\t' | '\\r' | '\\n' )+
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
            	    // InternalCallerSuffix.g:
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

    // $ANTLR start "RULE_ML_COMMENT"
    public final void mRULE_ML_COMMENT() throws RecognitionException {
        try {
            int _type = RULE_ML_COMMENT;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:290:17: ( '/*' ( options {greedy=false; } : . )* '*/' )
            // InternalCallerSuffix.g:290:19: '/*' ( options {greedy=false; } : . )* '*/'
            {
            match("/*"); 

            // InternalCallerSuffix.g:290:24: ( options {greedy=false; } : . )*
            loop3:
            do {
                int alt3=2;
                int LA3_0 = input.LA(1);

                if ( (LA3_0=='*') ) {
                    int LA3_1 = input.LA(2);

                    if ( (LA3_1=='/') ) {
                        alt3=2;
                    }
                    else if ( ((LA3_1>='\u0000' && LA3_1<='.')||(LA3_1>='0' && LA3_1<='\uFFFF')) ) {
                        alt3=1;
                    }


                }
                else if ( ((LA3_0>='\u0000' && LA3_0<=')')||(LA3_0>='+' && LA3_0<='\uFFFF')) ) {
                    alt3=1;
                }


                switch (alt3) {
            	case 1 :
            	    // InternalCallerSuffix.g:290:52: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop3;
                }
            } while (true);

            match("*/"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_ML_COMMENT"

    // $ANTLR start "RULE_SL_COMMENT"
    public final void mRULE_SL_COMMENT() throws RecognitionException {
        try {
            int _type = RULE_SL_COMMENT;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCallerSuffix.g:292:17: ( '//' (~ ( ( '\\n' | '\\r' ) ) )* ( ( '\\r' )? '\\n' )? )
            // InternalCallerSuffix.g:292:19: '//' (~ ( ( '\\n' | '\\r' ) ) )* ( ( '\\r' )? '\\n' )?
            {
            match("//"); 

            // InternalCallerSuffix.g:292:24: (~ ( ( '\\n' | '\\r' ) ) )*
            loop4:
            do {
                int alt4=2;
                int LA4_0 = input.LA(1);

                if ( ((LA4_0>='\u0000' && LA4_0<='\t')||(LA4_0>='\u000B' && LA4_0<='\f')||(LA4_0>='\u000E' && LA4_0<='\uFFFF')) ) {
                    alt4=1;
                }


                switch (alt4) {
            	case 1 :
            	    // InternalCallerSuffix.g:292:24: ~ ( ( '\\n' | '\\r' ) )
            	    {
            	    if ( (input.LA(1)>='\u0000' && input.LA(1)<='\t')||(input.LA(1)>='\u000B' && input.LA(1)<='\f')||(input.LA(1)>='\u000E' && input.LA(1)<='\uFFFF') ) {
            	        input.consume();

            	    }
            	    else {
            	        MismatchedSetException mse = new MismatchedSetException(null,input);
            	        recover(mse);
            	        throw mse;}


            	    }
            	    break;

            	default :
            	    break loop4;
                }
            } while (true);

            // InternalCallerSuffix.g:292:40: ( ( '\\r' )? '\\n' )?
            int alt6=2;
            int LA6_0 = input.LA(1);

            if ( (LA6_0=='\n'||LA6_0=='\r') ) {
                alt6=1;
            }
            switch (alt6) {
                case 1 :
                    // InternalCallerSuffix.g:292:41: ( '\\r' )? '\\n'
                    {
                    // InternalCallerSuffix.g:292:41: ( '\\r' )?
                    int alt5=2;
                    int LA5_0 = input.LA(1);

                    if ( (LA5_0=='\r') ) {
                        alt5=1;
                    }
                    switch (alt5) {
                        case 1 :
                            // InternalCallerSuffix.g:292:41: '\\r'
                            {
                            match('\r'); 

                            }
                            break;

                    }

                    match('\n'); 

                    }
                    break;

            }


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_SL_COMMENT"

    public void mTokens() throws RecognitionException {
        // InternalCallerSuffix.g:1:8: ( T__8 | T__9 | T__10 | T__11 | T__12 | T__13 | RULE_ID | RULE_WS | RULE_ML_COMMENT | RULE_SL_COMMENT )
        int alt7=10;
        alt7 = dfa7.predict(input);
        switch (alt7) {
            case 1 :
                // InternalCallerSuffix.g:1:10: T__8
                {
                mT__8(); 

                }
                break;
            case 2 :
                // InternalCallerSuffix.g:1:15: T__9
                {
                mT__9(); 

                }
                break;
            case 3 :
                // InternalCallerSuffix.g:1:20: T__10
                {
                mT__10(); 

                }
                break;
            case 4 :
                // InternalCallerSuffix.g:1:26: T__11
                {
                mT__11(); 

                }
                break;
            case 5 :
                // InternalCallerSuffix.g:1:32: T__12
                {
                mT__12(); 

                }
                break;
            case 6 :
                // InternalCallerSuffix.g:1:38: T__13
                {
                mT__13(); 

                }
                break;
            case 7 :
                // InternalCallerSuffix.g:1:44: RULE_ID
                {
                mRULE_ID(); 

                }
                break;
            case 8 :
                // InternalCallerSuffix.g:1:52: RULE_WS
                {
                mRULE_WS(); 

                }
                break;
            case 9 :
                // InternalCallerSuffix.g:1:60: RULE_ML_COMMENT
                {
                mRULE_ML_COMMENT(); 

                }
                break;
            case 10 :
                // InternalCallerSuffix.g:1:76: RULE_SL_COMMENT
                {
                mRULE_SL_COMMENT(); 

                }
                break;

        }

    }


    protected DFA7 dfa7 = new DFA7(this);
    static final String DFA7_eotS =
        "\3\uffff\1\6\1\13\1\14\3\uffff\2\6\4\uffff\3\6\1\24\1\6\1\uffff\1\26\1\uffff";
    static final String DFA7_eofS =
        "\27\uffff";
    static final String DFA7_minS =
        "\1\11\2\uffff\1\151\2\141\2\uffff\1\52\1\162\1\156\4\uffff\2\145\1\143\1\141\1\164\1\uffff\1\141\1\uffff";
    static final String DFA7_maxS =
        "\1\u00e9\2\uffff\1\157\2\u00e9\2\uffff\1\57\1\162\1\156\4\uffff\2\145\1\143\1\u00e9\1\164\1\uffff\1\u00e9\1\uffff";
    static final String DFA7_acceptS =
        "\1\uffff\1\1\1\2\3\uffff\1\7\1\10\3\uffff\1\5\1\6\1\11\1\12\5\uffff\1\4\1\uffff\1\3";
    static final String DFA7_specialS =
        "\27\uffff}>";
    static final String[] DFA7_transitionS = {
            "\2\7\2\uffff\1\7\22\uffff\1\7\1\1\15\uffff\1\10\17\uffff\1\2\41\uffff\1\4\1\5\1\6\1\3\26\6\156\uffff\1\6",
            "",
            "",
            "\1\11\5\uffff\1\12",
            "\32\6\156\uffff\1\6",
            "\32\6\156\uffff\1\6",
            "",
            "",
            "\1\15\4\uffff\1\16",
            "\1\17",
            "\1\20",
            "",
            "",
            "",
            "",
            "\1\21",
            "\1\22",
            "\1\23",
            "\32\6\156\uffff\1\6",
            "\1\25",
            "",
            "\32\6\156\uffff\1\6",
            ""
    };

    static final short[] DFA7_eot = DFA.unpackEncodedString(DFA7_eotS);
    static final short[] DFA7_eof = DFA.unpackEncodedString(DFA7_eofS);
    static final char[] DFA7_min = DFA.unpackEncodedStringToUnsignedChars(DFA7_minS);
    static final char[] DFA7_max = DFA.unpackEncodedStringToUnsignedChars(DFA7_maxS);
    static final short[] DFA7_accept = DFA.unpackEncodedString(DFA7_acceptS);
    static final short[] DFA7_special = DFA.unpackEncodedString(DFA7_specialS);
    static final short[][] DFA7_transition;

    static {
        int numStates = DFA7_transitionS.length;
        DFA7_transition = new short[numStates][];
        for (int i=0; i<numStates; i++) {
            DFA7_transition[i] = DFA.unpackEncodedString(DFA7_transitionS[i]);
        }
    }

    class DFA7 extends DFA {

        public DFA7(BaseRecognizer recognizer) {
            this.recognizer = recognizer;
            this.decisionNumber = 7;
            this.eot = DFA7_eot;
            this.eof = DFA7_eof;
            this.min = DFA7_min;
            this.max = DFA7_max;
            this.accept = DFA7_accept;
            this.special = DFA7_special;
            this.transition = DFA7_transition;
        }
        public String getDescription() {
            return "1:1: Tokens : ( T__8 | T__9 | T__10 | T__11 | T__12 | T__13 | RULE_ID | RULE_WS | RULE_ML_COMMENT | RULE_SL_COMMENT );";
        }
    }
 

}