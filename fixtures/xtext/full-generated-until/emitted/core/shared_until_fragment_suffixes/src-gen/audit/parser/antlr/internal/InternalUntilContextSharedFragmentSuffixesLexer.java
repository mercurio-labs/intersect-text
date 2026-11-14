package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalUntilContextSharedFragmentSuffixesLexer extends Lexer {
    public static final int RULE_BLOCK=5;
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalUntilContextSharedFragmentSuffixesLexer() {;} 
    public InternalUntilContextSharedFragmentSuffixesLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalUntilContextSharedFragmentSuffixesLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalUntilContextSharedFragmentSuffixes.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalUntilContextSharedFragmentSuffixes.g:98:12: ( ( RULE_BLOCK 'x' | RULE_BLOCK 'y' ) )
            // InternalUntilContextSharedFragmentSuffixes.g:98:14: ( RULE_BLOCK 'x' | RULE_BLOCK 'y' )
            {
            // InternalUntilContextSharedFragmentSuffixes.g:98:14: ( RULE_BLOCK 'x' | RULE_BLOCK 'y' )
            int alt1=2;
            alt1 = dfa1.predict(input);
            switch (alt1) {
                case 1 :
                    // InternalUntilContextSharedFragmentSuffixes.g:98:15: RULE_BLOCK 'x'
                    {
                    mRULE_BLOCK(); 
                    match('x'); 

                    }
                    break;
                case 2 :
                    // InternalUntilContextSharedFragmentSuffixes.g:98:30: RULE_BLOCK 'y'
                    {
                    mRULE_BLOCK(); 
                    match('y'); 

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
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_BLOCK"
    public final void mRULE_BLOCK() throws RecognitionException {
        try {
            // InternalUntilContextSharedFragmentSuffixes.g:100:21: ( '@' ( options {greedy=false; } : . )* ( 'ab' | 'abc' ) )
            // InternalUntilContextSharedFragmentSuffixes.g:100:23: '@' ( options {greedy=false; } : . )* ( 'ab' | 'abc' )
            {
            match('@'); 
            // InternalUntilContextSharedFragmentSuffixes.g:100:27: ( options {greedy=false; } : . )*
            loop2:
            do {
                int alt2=2;
                int LA2_0 = input.LA(1);

                if ( (LA2_0=='a') ) {
                    int LA2_1 = input.LA(2);

                    if ( (LA2_1=='b') ) {
                        alt2=2;
                    }
                    else if ( ((LA2_1>='\u0000' && LA2_1<='a')||(LA2_1>='c' && LA2_1<='\uFFFF')) ) {
                        alt2=1;
                    }


                }
                else if ( ((LA2_0>='\u0000' && LA2_0<='`')||(LA2_0>='b' && LA2_0<='\uFFFF')) ) {
                    alt2=1;
                }


                switch (alt2) {
            	case 1 :
            	    // InternalUntilContextSharedFragmentSuffixes.g:100:55: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop2;
                }
            } while (true);

            // InternalUntilContextSharedFragmentSuffixes.g:100:59: ( 'ab' | 'abc' )
            int alt3=2;
            int LA3_0 = input.LA(1);

            if ( (LA3_0=='a') ) {
                int LA3_1 = input.LA(2);

                if ( (LA3_1=='b') ) {
                    int LA3_2 = input.LA(3);

                    if ( (LA3_2=='c') ) {
                        alt3=2;
                    }
                    else {
                        alt3=1;}
                }
                else {
                    NoViableAltException nvae =
                        new NoViableAltException("", 3, 1, input);

                    throw nvae;
                }
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 3, 0, input);

                throw nvae;
            }
            switch (alt3) {
                case 1 :
                    // InternalUntilContextSharedFragmentSuffixes.g:100:60: 'ab'
                    {
                    match("ab"); 


                    }
                    break;
                case 2 :
                    // InternalUntilContextSharedFragmentSuffixes.g:100:65: 'abc'
                    {
                    match("abc"); 


                    }
                    break;

            }


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_BLOCK"

    public void mTokens() throws RecognitionException {
        // InternalUntilContextSharedFragmentSuffixes.g:1:8: ( RULE_VALUE )
        // InternalUntilContextSharedFragmentSuffixes.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


    protected DFA1 dfa1 = new DFA1(this);
    static final String DFA1_eotS =
        "\6\uffff\1\10\1\11\2\uffff";
    static final String DFA1_eofS =
        "\12\uffff";
    static final String DFA1_minS =
        "\1\100\7\0\2\uffff";
    static final String DFA1_maxS =
        "\1\100\7\uffff\2\uffff";
    static final String DFA1_acceptS =
        "\10\uffff\1\1\1\2";
    static final String DFA1_specialS =
        "\1\uffff\1\0\1\4\1\1\1\6\1\3\1\5\1\2\2\uffff}>";
    static final String[] DFA1_transitionS = {
            "\1\1",
            "\141\3\1\2\uff9e\3",
            "\141\3\1\2\1\4\uff9d\3",
            "\141\3\1\2\uff9e\3",
            "\141\3\1\2\1\3\1\5\24\3\1\6\1\7\uff86\3",
            "\141\3\1\2\26\3\1\6\1\7\uff86\3",
            "\141\3\1\2\uff9e\3",
            "\141\3\1\2\uff9e\3",
            "",
            ""
    };

    static final short[] DFA1_eot = DFA.unpackEncodedString(DFA1_eotS);
    static final short[] DFA1_eof = DFA.unpackEncodedString(DFA1_eofS);
    static final char[] DFA1_min = DFA.unpackEncodedStringToUnsignedChars(DFA1_minS);
    static final char[] DFA1_max = DFA.unpackEncodedStringToUnsignedChars(DFA1_maxS);
    static final short[] DFA1_accept = DFA.unpackEncodedString(DFA1_acceptS);
    static final short[] DFA1_special = DFA.unpackEncodedString(DFA1_specialS);
    static final short[][] DFA1_transition;

    static {
        int numStates = DFA1_transitionS.length;
        DFA1_transition = new short[numStates][];
        for (int i=0; i<numStates; i++) {
            DFA1_transition[i] = DFA.unpackEncodedString(DFA1_transitionS[i]);
        }
    }

    class DFA1 extends DFA {

        public DFA1(BaseRecognizer recognizer) {
            this.recognizer = recognizer;
            this.decisionNumber = 1;
            this.eot = DFA1_eot;
            this.eof = DFA1_eof;
            this.min = DFA1_min;
            this.max = DFA1_max;
            this.accept = DFA1_accept;
            this.special = DFA1_special;
            this.transition = DFA1_transition;
        }
        public String getDescription() {
            return "98:14: ( RULE_BLOCK 'x' | RULE_BLOCK 'y' )";
        }
        public int specialStateTransition(int s, IntStream _input) throws NoViableAltException {
            IntStream input = _input;
        	int _s = s;
            switch ( s ) {
                    case 0 : 
                        int LA1_1 = input.LA(1);

                        s = -1;
                        if ( (LA1_1=='a') ) {s = 2;}

                        else if ( ((LA1_1>='\u0000' && LA1_1<='`')||(LA1_1>='b' && LA1_1<='\uFFFF')) ) {s = 3;}

                        if ( s>=0 ) return s;
                        break;
                    case 1 : 
                        int LA1_3 = input.LA(1);

                        s = -1;
                        if ( (LA1_3=='a') ) {s = 2;}

                        else if ( ((LA1_3>='\u0000' && LA1_3<='`')||(LA1_3>='b' && LA1_3<='\uFFFF')) ) {s = 3;}

                        if ( s>=0 ) return s;
                        break;
                    case 2 : 
                        int LA1_7 = input.LA(1);

                        s = -1;
                        if ( (LA1_7=='a') ) {s = 2;}

                        else if ( ((LA1_7>='\u0000' && LA1_7<='`')||(LA1_7>='b' && LA1_7<='\uFFFF')) ) {s = 3;}

                        else s = 9;

                        if ( s>=0 ) return s;
                        break;
                    case 3 : 
                        int LA1_5 = input.LA(1);

                        s = -1;
                        if ( (LA1_5=='a') ) {s = 2;}

                        else if ( (LA1_5=='x') ) {s = 6;}

                        else if ( (LA1_5=='y') ) {s = 7;}

                        else if ( ((LA1_5>='\u0000' && LA1_5<='`')||(LA1_5>='b' && LA1_5<='w')||(LA1_5>='z' && LA1_5<='\uFFFF')) ) {s = 3;}

                        if ( s>=0 ) return s;
                        break;
                    case 4 : 
                        int LA1_2 = input.LA(1);

                        s = -1;
                        if ( (LA1_2=='b') ) {s = 4;}

                        else if ( (LA1_2=='a') ) {s = 2;}

                        else if ( ((LA1_2>='\u0000' && LA1_2<='`')||(LA1_2>='c' && LA1_2<='\uFFFF')) ) {s = 3;}

                        if ( s>=0 ) return s;
                        break;
                    case 5 : 
                        int LA1_6 = input.LA(1);

                        s = -1;
                        if ( (LA1_6=='a') ) {s = 2;}

                        else if ( ((LA1_6>='\u0000' && LA1_6<='`')||(LA1_6>='b' && LA1_6<='\uFFFF')) ) {s = 3;}

                        else s = 8;

                        if ( s>=0 ) return s;
                        break;
                    case 6 : 
                        int LA1_4 = input.LA(1);

                        s = -1;
                        if ( (LA1_4=='c') ) {s = 5;}

                        else if ( (LA1_4=='a') ) {s = 2;}

                        else if ( (LA1_4=='x') ) {s = 6;}

                        else if ( (LA1_4=='y') ) {s = 7;}

                        else if ( ((LA1_4>='\u0000' && LA1_4<='`')||LA1_4=='b'||(LA1_4>='d' && LA1_4<='w')||(LA1_4>='z' && LA1_4<='\uFFFF')) ) {s = 3;}

                        if ( s>=0 ) return s;
                        break;
            }
            NoViableAltException nvae =
                new NoViableAltException(getDescription(), 1, _s, input);
            error(nvae);
            throw nvae;
        }
    }
 

}