package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalCardinalityNullableFragmentOptionalLocalEotLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int RULE_BODY=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalCardinalityNullableFragmentOptionalLocalEotLexer() {;} 
    public InternalCardinalityNullableFragmentOptionalLocalEotLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalCardinalityNullableFragmentOptionalLocalEotLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalCardinalityNullableFragmentOptionalLocalEot.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalCardinalityNullableFragmentOptionalLocalEot.g:98:12: ( RULE_BODY 'ab' )
            // InternalCardinalityNullableFragmentOptionalLocalEot.g:98:14: RULE_BODY 'ab'
            {
            mRULE_BODY(); 
            match("ab"); 


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_BODY"
    public final void mRULE_BODY() throws RecognitionException {
        try {
            // InternalCardinalityNullableFragmentOptionalLocalEot.g:100:20: ( ( 'a' )? )
            // InternalCardinalityNullableFragmentOptionalLocalEot.g:100:22: ( 'a' )?
            {
            // InternalCardinalityNullableFragmentOptionalLocalEot.g:100:22: ( 'a' )?
            int alt1=2;
            int LA1_0 = input.LA(1);

            if ( (LA1_0=='a') ) {
                alt1=1;
            }
            switch (alt1) {
                case 1 :
                    // InternalCardinalityNullableFragmentOptionalLocalEot.g:100:22: 'a'
                    {
                    match('a'); 

                    }
                    break;

            }


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_BODY"

    public void mTokens() throws RecognitionException {
        // InternalCardinalityNullableFragmentOptionalLocalEot.g:1:8: ( RULE_VALUE )
        // InternalCardinalityNullableFragmentOptionalLocalEot.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}