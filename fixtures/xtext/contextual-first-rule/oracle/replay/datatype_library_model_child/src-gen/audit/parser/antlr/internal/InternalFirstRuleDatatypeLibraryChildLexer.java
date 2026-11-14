package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFirstRuleDatatypeLibraryChildLexer extends Lexer {
    public static final int T__4=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalFirstRuleDatatypeLibraryChildLexer() {;} 
    public InternalFirstRuleDatatypeLibraryChildLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalFirstRuleDatatypeLibraryChildLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalFirstRuleDatatypeLibraryChild__.g"; }

    // $ANTLR start "T__4"
    public final void mT__4() throws RecognitionException {
        try {
            int _type = T__4;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalFirstRuleDatatypeLibraryChild__.g:11:6: ( 'a' )
            // InternalFirstRuleDatatypeLibraryChild__.g:11:8: 'a'
            {
            match('a'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "T__4"

    public void mTokens() throws RecognitionException {
        // InternalFirstRuleDatatypeLibraryChild__.g:1:8: ( T__4 )
        // InternalFirstRuleDatatypeLibraryChild__.g:1:10: T__4
        {
        mT__4(); 

        }


    }


 

}