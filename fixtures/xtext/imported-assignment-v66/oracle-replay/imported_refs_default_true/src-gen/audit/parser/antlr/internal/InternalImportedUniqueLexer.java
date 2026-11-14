package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalImportedUniqueLexer extends Lexer {
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int T__7=7;
    public static final int RULE_ID=4;
    public static final int T__6=6;
    public static final int RULE_WS=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalImportedUniqueLexer() {;} 
    public InternalImportedUniqueLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalImportedUniqueLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalImportedUnique.g"; }

    // $ANTLR start "T__6"
    public final void mT__6() throws RecognitionException {
        try {
            int _type = T__6;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalImportedUnique.g:11:6: ( 'nodes' )
            // InternalImportedUnique.g:11:8: 'nodes'
            {
            match("nodes"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__6"

    // $ANTLR start "T__7"
    public final void mT__7() throws RecognitionException {
        try {
            int _type = T__7;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalImportedUnique.g:12:6: ( ',' )
            // InternalImportedUnique.g:12:8: ','
            {
            match(','); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__7"

    // $ANTLR start "T__8"
    public final void mT__8() throws RecognitionException {
        try {
            int _type = T__8;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalImportedUnique.g:13:6: ( ';' )
            // InternalImportedUnique.g:13:8: ';'
            {
            match(';'); 

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
            // InternalImportedUnique.g:14:6: ( 'refs' )
            // InternalImportedUnique.g:14:8: 'refs'
            {
            match("refs"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__9"

    // $ANTLR start "RULE_ID"
    public final void mRULE_ID() throws RecognitionException {
        try {
            int _type = RULE_ID;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalImportedUnique.g:209:9: ( ( 'a' .. 'z' )+ )
            // InternalImportedUnique.g:209:11: ( 'a' .. 'z' )+
            {
            // InternalImportedUnique.g:209:11: ( 'a' .. 'z' )+
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
            	    // InternalImportedUnique.g:209:12: 'a' .. 'z'
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
            // InternalImportedUnique.g:211:9: ( ( ' ' | '\\t' | '\\r' | '\\n' )+ )
            // InternalImportedUnique.g:211:11: ( ' ' | '\\t' | '\\r' | '\\n' )+
            {
            // InternalImportedUnique.g:211:11: ( ' ' | '\\t' | '\\r' | '\\n' )+
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
            	    // InternalImportedUnique.g:
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
        // InternalImportedUnique.g:1:8: ( T__6 | T__7 | T__8 | T__9 | RULE_ID | RULE_WS )
        int alt3=6;
        alt3 = dfa3.predict(input);
        switch (alt3) {
            case 1 :
                // InternalImportedUnique.g:1:10: T__6
                {
                mT__6(); 

                }
                break;
            case 2 :
                // InternalImportedUnique.g:1:15: T__7
                {
                mT__7(); 

                }
                break;
            case 3 :
                // InternalImportedUnique.g:1:20: T__8
                {
                mT__8(); 

                }
                break;
            case 4 :
                // InternalImportedUnique.g:1:25: T__9
                {
                mT__9(); 

                }
                break;
            case 5 :
                // InternalImportedUnique.g:1:30: RULE_ID
                {
                mRULE_ID(); 

                }
                break;
            case 6 :
                // InternalImportedUnique.g:1:38: RULE_WS
                {
                mRULE_WS(); 

                }
                break;

        }

    }


    protected DFA3 dfa3 = new DFA3(this);
    static final String DFA3_eotS =
        "\1\uffff\1\5\2\uffff\1\5\2\uffff\5\5\1\16\1\17\2\uffff";
    static final String DFA3_eofS =
        "\20\uffff";
    static final String DFA3_minS =
        "\1\11\1\157\2\uffff\1\145\2\uffff\1\144\1\146\1\145\2\163\2\141\2\uffff";
    static final String DFA3_maxS =
        "\1\172\1\157\2\uffff\1\145\2\uffff\1\144\1\146\1\145\2\163\2\172\2\uffff";
    static final String DFA3_acceptS =
        "\2\uffff\1\2\1\3\1\uffff\1\5\1\6\7\uffff\1\4\1\1";
    static final String DFA3_specialS =
        "\20\uffff}>";
    static final String[] DFA3_transitionS = {
            "\2\6\2\uffff\1\6\22\uffff\1\6\13\uffff\1\2\16\uffff\1\3\45\uffff\15\5\1\1\3\5\1\4\10\5",
            "\1\7",
            "",
            "",
            "\1\10",
            "",
            "",
            "\1\11",
            "\1\12",
            "\1\13",
            "\1\14",
            "\1\15",
            "\32\5",
            "\32\5",
            "",
            ""
    };

    static final short[] DFA3_eot = DFA.unpackEncodedString(DFA3_eotS);
    static final short[] DFA3_eof = DFA.unpackEncodedString(DFA3_eofS);
    static final char[] DFA3_min = DFA.unpackEncodedStringToUnsignedChars(DFA3_minS);
    static final char[] DFA3_max = DFA.unpackEncodedStringToUnsignedChars(DFA3_maxS);
    static final short[] DFA3_accept = DFA.unpackEncodedString(DFA3_acceptS);
    static final short[] DFA3_special = DFA.unpackEncodedString(DFA3_specialS);
    static final short[][] DFA3_transition;

    static {
        int numStates = DFA3_transitionS.length;
        DFA3_transition = new short[numStates][];
        for (int i=0; i<numStates; i++) {
            DFA3_transition[i] = DFA.unpackEncodedString(DFA3_transitionS[i]);
        }
    }

    class DFA3 extends DFA {

        public DFA3(BaseRecognizer recognizer) {
            this.recognizer = recognizer;
            this.decisionNumber = 3;
            this.eot = DFA3_eot;
            this.eof = DFA3_eof;
            this.min = DFA3_min;
            this.max = DFA3_max;
            this.accept = DFA3_accept;
            this.special = DFA3_special;
            this.transition = DFA3_transition;
        }
        public String getDescription() {
            return "1:1: Tokens : ( T__6 | T__7 | T__8 | T__9 | RULE_ID | RULE_WS );";
        }
    }
 

}