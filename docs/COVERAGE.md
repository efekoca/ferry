# PHP upgrade coverage

Every backward incompatible change, deprecation, removed extension and removed ini setting listed in php-src's
`UPGRADING` files from PHP 7.0 to 8.5, and what Ferry does about it. Each covered item was checked by scanning a
minimal snippet with `ferry-cli scan --target 8.5`.

- **Covered**: Ferry reports code affected by the change.
- **Partial**: some forms are reported, others that could be found statically are not.
- **Gap**: could be found from the source but isn't reported yet.
- **Runtime only**: depends on values or types that are only known when the code runs.
- **Not applicable**: doesn't affect application code (C API, build options, SAPIs, internals).

The data lives in `docs/coverage.json`; this page is generated from it with
`node scripts/php-upgrading/render.mjs`.

| Version | Covered | Partial | Gap | Runtime only | Not applicable | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [7.0](#php-70) | 43 | 3 | 3 | 14 | 19 | 82 |
| [7.1](#php-71) | 8 | 0 | 3 | 13 | 4 | 28 |
| [7.2](#php-72) | 23 | 1 | 1 | 5 | 4 | 34 |
| [7.3](#php-73) | 14 | 0 | 2 | 12 | 6 | 34 |
| [7.4](#php-74) | 34 | 1 | 1 | 25 | 4 | 65 |
| [8.0](#php-80) | 122 | 2 | 1 | 48 | 11 | 184 |
| [8.1](#php-81) | 48 | 1 | 5 | 6 | 2 | 62 |
| [8.2](#php-82) | 11 | 0 | 0 | 7 | 0 | 18 |
| [8.3](#php-83) | 20 | 1 | 2 | 10 | 2 | 35 |
| [8.4](#php-84) | 69 | 3 | 0 | 22 | 5 | 99 |
| [8.5](#php-85) | 50 | 0 | 2 | 22 | 5 | 79 |
| **All** | **442** | **12** | **20** | **184** | **62** | **720** |

## PHP 7.0

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-7.0/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Changes to variable handling | Indirect variable, property and method references are now interpreted with left-to-right semantics. | Partial | `review.variable_variable` | $$foo['bar']['baz'] is flagged as a variable variable, but $foo->$bar['baz'], $foo->$bar['baz']() and Foo::$bar['baz']() are not reported. |
| Changes to variable handling | The global keyword now only accepts simple variables. | Covered | `review.variable_variable` | global $$foo->bar is reported as a variable variable. |
| Changes to variable handling | Parentheses around variables or function calls no longer have any influence on behavior. | Covered | `behavior.parenthesized_reference` | A function result wrapped in extra parentheses and passed to a by-reference built-in such as array_pop() is reported. |
| Changes to variable handling | Array elements or object properties that are automatically created during by-reference assignments will now result in a different order. | Runtime only |  | Ordering of elements auto-created by reference assignment only shows at runtime. |
| Changes to variable handling | https://wiki.php.net/rfc/uniform_variable_syntax | Not applicable |  | RFC reference only; the change is classified under item 0. |
| Changes to variable handling | https://wiki.php.net/rfc/abstract_syntax_tree | Not applicable |  | RFC reference only; the changes are classified under items 0, 6 and 7. |
| Changes to list() | list() will no longer assign variables in reverse order. | Covered | `behavior.list_assignment_order` | list() appending to the same array more than once is reported. |
| Changes to list() | Empty list() assignments are no longer allowed. | Covered | `removed.empty_list` | Empty list() and list(,,) assignments are reported. |
| Changes to list() | list() no longer supports unpacking strings (while previously this was only supported in some cases). | Runtime only |  | Whether list() unpacks a string depends on the runtime type of the right-hand side. |
| Changes to list() | https://wiki.php.net/rfc/abstract_syntax_tree#changes_to_list | Not applicable |  | RFC reference only; the change is classified under items 6 and 7. |
| Changes to list() | https://wiki.php.net/rfc/fix_list_behavior_inconsistency | Not applicable |  | RFC reference only; the change is classified under item 8. |
| Changes to foreach | Iteration with foreach() no longer has any effect on the internal array pointer, which can be accessed through the current()/next()/etc family of functions. | Gap |  | current($array) inside foreach ($array as &$val) is not reported. |
| Changes to foreach | When iterating arrays by-value, foreach will now always operate on a copy of the array, as such changes to the array during iteration will not influence iteration behavior. | Runtime only |  | Old behavior only appeared when the array was referenced elsewhere and modified during iteration. |
| Changes to foreach | When iterating arrays by-reference, modifications to the array will continue to influence the iteration. | Runtime only |  | By-reference iteration position changes depend on runtime modifications of the array. |
| Changes to foreach | Iteration of plain (non-Traversable) objects by-value or by-reference will behave like by-reference iteration of arrays. | Runtime only |  | Plain object iteration position handling depends on runtime modifications. |
| Changes to foreach | Iteration of Traversable objects remains unchanged. | Not applicable |  | Traversable iteration is unchanged. |
| Changes to parameter handling | It is no longer possible to define two function parameters with the same name. | Covered | `removed.duplicate_parameter` | function foo($a, $b, $unused, $unused) {} is reported. |
| Changes to parameter handling | The func_get_arg() and func_get_args() functions will no longer return the original value that was passed to a parameter and will instead provide the current value (which might have been modified). | Gap |  | func_get_arg(0) after the parameter is modified ($x++) is not reported. |
| Changes to parameter handling | Similarly exception backtraces will no longer display the original value that was passed to a function and show the modified value instead. | Not applicable |  | Only affects backtrace display, not program behavior. |
| Changes to integer handling | Invalid octal literals (containing digits larger than 7) now produce compile errors. | Covered | `removed.invalid_octal` | Octal literals containing 8 or 9, such as 0781, are reported. |
| Changes to integer handling | Bitwise shifts by negative numbers will now throw an ArithmeticError: | Covered | `removed.negative_shift` | Shifts by a negative integer literal, such as 1 >> -1, are reported. |
| Changes to integer handling | Left bitwise shifts by a number of bits beyond the bit width of an integer will always result in 0: | Covered | `behavior.wide_shift` | Shifts by an integer literal of 64 or more are reported. |
| Changes to integer handling | Similarly right bitwise shifts by a number of bits beyond the bit width of an integer will always result in 0 or -1 (depending on sign): | Covered | `behavior.wide_shift` | Shifts by an integer literal of 64 or more are reported. |
| Changes to string handling | Strings that contain hexadecimal numbers are no longer considered to be numeric and don't receive special treatment anymore. | Covered | `behavior.hex_numeric_string` | Hex strings such as '0x1A' passed to is_numeric() or compared with == are reported. |
| Changes to string handling | Due to the addition of the Unicode Codepoint Escape Syntax for double-quoted strings and heredocs, "\u{" followed by an invalid sequence will now result in an error: | Covered | `removed.invalid_unicode_escape` | Double-quoted strings and heredocs with an invalid \u{ escape are reported. |
| Changes to string handling | https://wiki.php.net/rfc/remove_hex_support_in_numeric_strings | Not applicable |  | RFC reference only; the change is classified under item 23. |
| Changes to string handling | https://wiki.php.net/rfc/unicode_escape | Not applicable |  | RFC reference only; the change is classified under item 24. |
| Changes to error handling | There are now two exception classes: | Covered | `removed.exception_handler_type` | set_exception_handler(function (Exception $e) {...}) is reported. |
| Changes to error handling | Some fatal errors and recoverable fatal errors now throw an Error instead. | Runtime only |  | Which errors become Error and escape catch (Exception) depends on runtime failures. |
| Changes to error handling | Parser errors now generate a ParseError that extends Error. | Covered | `review.eval` | eval() usage is reported for review of ParseError handling. |
| Changes to error handling | Constructors of internal classes will now always throw an exception on failure. | Runtime only |  | Whether an internal constructor fails depends on runtime arguments. |
| Changes to error handling | The error level of some E_STRICT notices has been changed. | Not applicable |  | Error level reclassification of E_STRICT notices does not change application source. |
| Changes to error handling | https://wiki.php.net/rfc/engine_exceptions_for_php7 | Not applicable |  | RFC reference only; the change is classified under item 28. |
| Changes to error handling | https://wiki.php.net/rfc/throwable-interface | Not applicable |  | RFC reference only; the change is classified under item 27. |
| Changes to error handling | https://wiki.php.net/rfc/internal_constructor_behaviour | Not applicable |  | RFC reference only; the change is classified under item 30. |
| Changes to error handling | https://wiki.php.net/rfc/reclassify_e_strict | Not applicable |  | RFC reference only; the change is classified under item 31. |
| Other language changes | Removed support for static calls to non-static methods from an incompatible $this context. | Covered | `removed.static_call_to_instance_method` | A::test() called from unrelated class B is reported. |
| Other language changes | It is no longer possible to use the following class, interface and trait names (case-insensitive): | Covered | `removed.reserved_name`<br>`removed.reserved_alias` | Class declarations, use aliases and class_alias() with the reserved name are reported. |
| Other language changes | The yield language construct no longer requires parentheses when used in an expression context. | Gap |  | echo yield -1; and yield $foo or die; precedence change is not reported. |
| Other language changes | Removed ASP (&lt;%) and script (&lt;script language=php>) tags. | Partial | `removed.script_tag` | &lt;script language="php"> in HTML is reported; ASP tags (&lt;% %>) are not, because the same markers appear in other template languages. |
| Other language changes | Removed support for assigning the result of new by reference. | Covered | `removed.assign_new_by_ref` | $a = &new Foo is reported. |
| Other language changes | Removed support for scoped calls to non-static methods from an incompatible $this context. | Covered | `removed.static_call_to_instance_method` | Scoped call A::test() from an incompatible context is reported. |
| Other language changes | Removed support for #-style comments in ini files. | Covered | `removed.ini_hash_comment` | Lines starting with # in php.ini and .user.ini are reported. |
| Other language changes | $HTTP_RAW_POST_DATA is no longer available. | Covered | `removed.magic_variable` | $HTTP_RAW_POST_DATA is reported. |
| Standard library changes | substr() now returns an empty string instead of FALSE when the truncation happens on boundaries. | Covered | `behavior.substr_false` | Strict comparisons of a substr() call with false are reported. |
| Standard library changes | call_user_method() and call_user_method_array() no longer exists. | Covered | `removed.call_user_method`<br>`removed.call_user_method_array` | call_user_method() and call_user_method_array() are reported. |
| Standard library changes | ob_start() no longer issues an E_ERROR, but instead an E_RECOVERABLE_ERROR in case an output buffer is created in an output buffer handler. | Runtime only |  | Depends on ob_start() being called inside an output handler at runtime. |
| Standard library changes | The internal sorting algorithm has been improved, what may result in different sort order of elements that compare as equal. | Runtime only |  | Sort order of equal elements depends on data. |
| Standard library changes | Removed dl() function on fpm-fcgi. | Runtime only |  | dl() is only unavailable under the fpm-fcgi SAPI, decided at deployment time; dl() calls are not reported. |
| Standard library changes | setcookie() with an empty cookie name now issues a WARNING and doesn't send an empty set-cookie header line anymore. | Covered | `behavior.setcookie_empty_name` | setcookie('', 'v') is reported. |
| Curl | Removed support for disabling the CURLOPT_SAFE_UPLOAD option. | Covered | `removed.curl_safe_upload_off` | curl_setopt($ch, CURLOPT_SAFE_UPLOAD, false) is reported. |
| Curl | curl_getinfo($ch, CURLINFO_CERTINFO) returns certificate Subject and Issuer as a string (PHP >= 5.6.25) | Covered | `behavior.curl_certinfo_format` | curl_getinfo() with CURLINFO_CERTINFO is reported. |
| Date | Removed $is_dst parameter from mktime() and gmmktime(). | Covered | `removed.builtin_arity` | mktime() and gmmktime() with the 7th $is_dst argument are reported. |
| Date | dba_delete() now returns false if the key was not found for the inifile handler, too. | Runtime only |  | dba_delete() return value depends on the handler and key at runtime. |
| Date | Requires libgmp version 4.2 or newer now. | Not applicable |  | libgmp build requirement. |
| Date | gmp_setbit() and gmp_clrbit() now return FALSE for negative indices, making them consistent with other GMP functions. | Covered | `removed.gmp_setbit_negative` | gmp_setbit($g, -1) is reported. |
| Intl | Removed deprecated aliases datefmt_set_timezone_id() and IntlDateFormatter::setTimeZoneID(). | Partial | `removed.datefmt_set_timezone_id` | datefmt_set_timezone_id() is reported, but IntlDateFormatter::setTimeZoneID() is not, even on a new IntlDateFormatter receiver. |
| libxml | Added LIBXML_BIGLINES parser option. | Not applicable |  | New libxml constant added. |
| libxml | Removed deprecated mcrypt_generic_end() alias in favor of mcrypt_generic_deinit(). | Covered | `removed.mcrypt` | mcrypt_generic_end() is reported. |
| libxml | Removed deprecated mcrypt_ecb(), mcrypt_cbc(), mcrypt_cfb() and mcrypt_ofb() functions in favor of mcrypt_encrypt() and mcrypt_decrypt() with an MCRYPT_MODE_* flag. | Covered | `removed.mcrypt` | mcrypt_ecb() and mcrypt_cbc() are reported. |
| libxml | session_start() accepts all INI settings as array. | Not applicable |  | New session_start() option support; no break. |
| libxml | Save handler accepts validate_sid(), update_timestamp() which validates session ID existence, updates timestamp of session data. | Not applicable |  | New save handler callbacks; old handlers keep working. |
| libxml | SessionUpdateTimestampHandlerInterface is added. | Not applicable |  | New interface added. |
| libxml | session.lazy_write(default=On) INI setting enables only write session data when session data is updated. | Not applicable |  | New INI setting with a compatible default. |
| libxml | session_regenerate_id() saves current $_SESSION before creating new session ID. | Runtime only |  | Session save timing on regeneration only shows at runtime. |
| libxml | Removed opcache.load_comments configuration directive. | Covered | `removed.builtin_ini` | opcache.load_comments is reported in ini files and ini_set(). |
| OpenSSL | Removed the "rsa_key_size" SSL context option in favor of automatically setting the appropriate size given the negotiated crypto algorithm. | Covered | `removed.ssl_context_option` | rsa_key_size used as an array key is reported. |
| OpenSSL | Removed "CN_match" and "SNI_server_name" SSL context options. | Covered | `removed.ssl_context_option` | CN_match and SNI_server_name used as array keys are reported. |
| PCRE | Removed support for /e (PREG_REPLACE_EVAL) modifier. | Covered | `removed.preg_eval_modifier` | preg_replace('/a/e', ...) is reported. |
| PDO_pgsql | Removed PGSQL_ATTR_DISABLE_NATIVE_PREPARED_STATEMENT attribute in favor of ATTR_EMULATE_PREPARES. | Covered | `deprecated.pdo_driver_constant` | PDO::PGSQL_ATTR_DISABLE_NATIVE_PREPARED_STATEMENT is reported. |
| Standard | Removed string category support in setlocale(). | Covered | `removed.setlocale_string_category` | setlocale('LC_ALL', 'en_US') is reported. |
| Standard | Removed set_magic_quotes_runtime() and its alias magic_quotes_runtime(). | Covered | `removed.set_magic_quotes_runtime`<br>`removed.magic_quotes_runtime` | set_magic_quotes_runtime() and magic_quotes_runtime() are reported. |
| JSON | Rejected RFC 7159 incompatible number formats in json_decode string - top level (07, 0xff, .1, -.1) and all levels ([1.], [1.e1]) | Runtime only |  | Depends on the JSON input data. |
| JSON | Calling json_decode with 1st argument equal to empty PHP string or value that after casting to string is empty string (NULL, FALSE) results in JSON syntax error. | Runtime only |  | Depends on json_decode() input being empty at runtime. |
| Stream | Removed set_socket_blocking() in favor of its alias stream_set_blocking(). | Covered | `removed.set_socket_blocking` | set_socket_blocking() is reported. |
| XML | xml_set_object() now requires to manually unset the $parser when finished, to avoid memory leaks. | Covered | `deprecated.xml_set_object` | xml_set_object() is reported (via its 8.4 deprecation rule). |
| XSL | Removed xsl.security_prefs ini option. | Covered | `removed.builtin_ini` | xsl.security_prefs is reported in ini files and ini_set(). |
| Deprecated Functionality | PHP 4 style constructors, where the constructor name is the same as the class name, are now deprecated. | Covered | `removed.php4_constructor` | class Foo { function Foo() {} } is reported. |
| Deprecated Functionality | Static calls to non-static methods are now deprecated. | Covered | `removed.static_call_to_instance_method` | A::m() on a non-static method is reported. |
| Deprecated Functionality | The "capture_session_meta" SSL context option is now deprecated. | Covered | `deprecated.ssl_capture_session_meta` | capture_session_meta used as an array key is reported. |
| Changes to INI File Handling | Removed asp_tags ini directive. | Covered | `removed.ini_setting` | asp_tags=On in php.ini and .htaccess is reported. |
| Changes to INI File Handling | Removed always_populate_raw_post_data ini directive. | Covered | `removed.builtin_ini` | always_populate_raw_post_data is reported in ini files and ini_set(). |

## PHP 7.1

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-7.1/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | 'void' can no longer be used as the name of a class, interface, or trait. | Covered | `removed.reserved_name` | Class declarations, use aliases and class_alias() with the reserved name are reported. |
| Core | 'iterable' can no longer be used as the name of a class, interface, or trait. | Covered | `removed.reserved_name`<br>`removed.reserved_alias` | Class declarations, use aliases and class_alias() with the reserved name are reported. |
| Core | (int), intval() where $base is 10 or unspecified, settype(), decbin(), decoct(), dechex(), integer operators and other conversions now always respect scientific notation in numeric strings. | Runtime only |  | Depends on numeric string values at runtime. |
| Core | The ASCII 0x7F Delete control character is no longer permitted in unquoted identifiers in source code. | Covered | `removed.delete_in_identifier` | A 0x7F byte in code outside strings and comments is reported. |
| Core | The following functions may no longer be called dynamically using $func(), call_user_func(), array_map() or similar: | Gap |  | Dynamic calls like $f = 'extract'; $f(), call_user_func('compact', ...) and array_map('func_get_args', ...) are not reported. |
| Core | If the error_log is set to syslog, the PHP error levels are mapped to the syslog error levels. | Not applicable |  | Syslog level mapping is a logging configuration detail. |
| Core | Don't call destructors of incompletely constructed objects, even if they are kept referenced. | Runtime only |  | Depends on a constructor failing at runtime. |
| Core | call_user_func() will now consistently throw a warning if a function with reference arguments is called. | Runtime only |  | Depends on the callee's by-reference signature at runtime. |
| Core | rand() and srand() are now aliases of mt_rand() and mt_srand(). | Gap |  | Seeded srand(42)/rand() sequences whose output changed are not reported. |
| Core | Fixes to random number generators mean that mt_rand() now produces a different sequence of outputs to previous versions. | Gap |  | mt_srand(5) followed by mt_rand() relying on a deterministic sequence is not reported. |
| Core | URL rewriter has been improved. | Runtime only |  | URL rewriter output depends on runtime INI settings and output. |
| Core | Calling a function with less arguments than mandatory declared ones in signature now issues a Fatal Error (Error Exception) instead of a Warning. | Covered | `removed.too_few_arguments` | Calling f($a, $b) as f(1) is reported. |
| Core | The error message for E_RECOVERABLE errors has been changed from "Catchable fatal error" to "Recoverable fatal error". | Not applicable |  | Error message wording change. |
| Core | The empty index operator (e.g. $str[] = $x) is not supported for strings anymore, and throws a fatal error instead of silently converting to array. | Runtime only |  | Whether $x[] = ... targets a string depends on the runtime type. |
| Core | Array elements or object properties that are automatically created during by-reference assignments will now result in a different order. | Runtime only |  | Ordering of elements auto-created by reference assignment only shows at runtime. |
| Core | The allowed_classes element of the $options parameter of unserialize() is now strictly typed, i.e. if anything other than an array or a boolean is given, unserialize() returns FALSE and issues an… | Covered | `behavior.unserialize_allowed_classes_type` | A string or number given for allowed_classes is reported. |
| Core | $this, autoglobals, and variables with the same name as a parameter can no longer be bound to a closure via the use construct. | Covered | `removed.closure_use_variable` | Closures binding $this, a superglobal or a variable named like a parameter with use() are reported. |
| JSON | The serialize_precision is used instead of precision when encoding double values. | Runtime only |  | Float encoding output depends on values and serialize_precision. |
| JSON | An empty key is decoded as an empty property name instead of using _empty_ property name when decoding object to stdClass. | Runtime only |  | Depends on JSON data containing empty keys. |
| JSON | When calling json_encode with JSON_UNESCAPED_UNICODE option, U+2028 and U+2029 are escaped. | Runtime only |  | Depends on data containing U+2028/U+2029. |
| mbstring | mb_ereg() and mb_eregi() will now set the $regs argument to an empty array, if nothing matched. | Runtime only |  | Depends on match results at runtime. |
| OpenSSL | Dropped sslv2 stream. | Covered | `removed.sslv2_stream` | Literal sslv2:// URLs are reported. |
| Session | Session ID is generated from CSPRNG directly. | Runtime only |  | Session ID length depends on runtime INI configuration. |
| Session | Following INIs are removed . | Covered | `removed.builtin_ini` | Each of the four settings is reported in php.ini, .user.ini, .htaccess and ini_set(). |
| Session | New INIs and defaults . | Not applicable |  | New INI settings with defaults. |
| Session | Length of old session ID string is determined as follows . | Not applicable |  | Explanation of old session ID length computation. |
| Session | session_start() returns FALSE and no longer initializes $_SESSION when it failed to start session. | Runtime only |  | Depends on session_start() failing at runtime. |
| Reflection | The behavior of ReflectionMethod::invoke() and ::invokeArgs() has been aligned, which causes slightly different behavior than before for some pathological cases. | Runtime only |  | Only pathological invoke() cases at runtime. |

## PHP 7.2

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-7.2/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | gettype() will now return "resource (closed)" instead of "unknown type" for closed resources. | Covered | `behavior.gettype_closed_resource` | Comparisons with the literal 'unknown type' are reported. |
| Core | is_object() will now return true for objects of class __PHP_Incomplete_Class. | Runtime only |  | Depends on objects being __PHP_Incomplete_Class at runtime. |
| Core | Support for Netware operating systems has been removed. | Not applicable |  | Netware OS support removal. |
| Core | Casting arrays to objects (with (object) or settype()) will now convert integer keys to string property names. | Runtime only |  | Depends on array keys at runtime. |
| Core | Casting objects to arrays (with (array) or settype()), and retrieving object properties in an array with get_object_vars(), will now convert numeric string property names (that is, property names… | Runtime only |  | Depends on property names at runtime. |
| Core | Unqualified references to undefined constants now generate a Warning instead of a notice. | Partial | `removed.bareword_key` | Bareword array keys $a[key] are reported, but other undefined bare constants like echo FOO_UNDEF are not. |
| Core | Minimum supported Windows versions are Windows 7/Server 2008 R2. | Not applicable |  | Windows platform requirement. |
| Core | Initial trait property value compatibility check will no longer perform any casts. | Covered | `removed.trait_property_default` | A class redeclaring a property of a trait from the same file with a different literal default is reported. |
| Core | "object" (in any case) can no longer be used as a class name. | Covered | `removed.reserved_name` | class Object {} is reported. |
| BCMath | The bcmod() function no longer truncates fractional numbers to integers. | Runtime only |  | Depends on bcmod() operands being fractional at runtime. |
| Hash | The hash_hmac(), hash_hmac_file(), hash_pbkdf2() and hash_init() (with HASH_HMAC) functions no longer accept non-cryptographic hashes. | Covered | `removed.hash_hmac_non_crypto` | hash_hmac('crc32', $d, $k) is reported. |
| JSON | The json_decode() option JSON_OBJECT_AS_ARRAY is used if the second parameter (assoc) is null. | Covered | `behavior.json_decode_null_assoc` | json_decode($s, null, 512, JSON_OBJECT_AS_ARRAY) is reported. |
| SAPI | Starting with 7.2.34, incoming cookie names are not url-decoded. | Not applicable |  | SAPI cookie name decoding of incoming requests. |
| Session | Removed register_globals related code and "!" can be used as $_SESSION key name. | Not applicable |  | Internal removal of register_globals code. |
| Session | Session is made to manage session status correctly and prevents invalid operations. | Runtime only |  | Depends on session state at runtime. |
| Standard | Sequences generated by mt_rand() and rand() for a specific seed may differ from PHP 7.1 on 64-bit machines. | Gap |  | Seeded mt_srand()/srand() sequences relied upon for determinism are not reported. |
| Core | The track_errors ini directive has been deprecated. | Covered | `removed.ini_setting`<br>`deprecated.ini_setting`<br>`removed.magic_variable` | track_errors in php.ini and ini_set(), and $php_errormsg, are reported. |
| Core | The __autoload() mechanism has been deprecated, use spl_autoload_register() instead. | Covered | `removed.__autoload` | function __autoload() is reported. |
| Core | The (unset) cast has been deprecated. | Covered | `removed.unset_cast` | (unset) cast is reported. |
| Core | The create_function() function has been deprecated, use anonymous functions instead. | Covered | `removed.create_function` | create_function() is reported. |
| Core | The each() function has been deprecated, use a foreach loop instead. | Covered | `removed.each` | each() is reported. |
| Core | The $errcontext error handler argument has been deprecated. | Covered | `removed.error_handler_context` | Error handler with a fifth $errcontext parameter is reported. |
| EXIF | The read_exif_data() alias has been deprecated, use exif_read_data() instead. | Covered | `removed.read_exif_data` | read_exif_data() is reported. |
| GD | png2wbmp() and jpeg2wbmp() have been deprecated. | Covered | `removed.png2wbmp`<br>`removed.jpeg2wbmp` | png2wbmp() and jpeg2wbmp() are reported. |
| GMP | The gmp_random() function has been deprecated, use gmp_random_bits() or gmp_random_range() instead. | Covered | `removed.gmp_random` | gmp_random() is reported. |
| Intl | INTL_IDNA_VARIANT_2003 has been deprecated; use INTL_IDNA_VARIANT_UTS46 instead. | Covered | `removed.intl_idna_variant_2003` | INTL_IDNA_VARIANT_2003 is reported. |
| Mbstring | The mbstring.func_overload ini directive has been deprecated. | Covered | `deprecated.ini_setting` | mbstring.func_overload in php.ini is reported. |
| Standard | Calling parse_str() without the result argument has been deprecated. | Covered | `removed.parse_str_one_arg`<br>`removed.builtin_arity` | parse_str($s) with one argument is reported. |
| Standard | Calling assert() with a string argument has been deprecated, use an ordinary expression instead. | Covered | `removed.assert_string` | assert('$a > 0') is reported. |
| Mcrypt | The deprecated mcrypt extension has been moved to PECL. | Covered | `removed.mcrypt` | mcrypt_encrypt() is reported. |
| Mcrypt | libmcrypt has not been maintained since 2007 and the continued use of this extension is strongly discouraged. | Covered | `removed.mcrypt` | Continuation of the mcrypt removal; mcrypt_* calls are reported. |
| Mcrypt | Users are advised to use alternatives such as OpenSSL or libsodium. | Covered | `removed.mcrypt` | Continuation of the mcrypt removal; mcrypt_* calls are reported. |
| Changes to INI File Handling | This INI directive has been removed. | Covered | `removed.builtin_ini` | sql.safe_mode is reported in php.ini, .user.ini, .htaccess and ini_set(). |
| Changes to INI File Handling | This INI directive has been removed. | Covered | `removed.builtin_ini` | opcache.fast_shutdown is reported in php.ini, .user.ini, .htaccess and ini_set(). |

## PHP 7.3

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-7.3/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Backward Incompatible Changes | The ext_skel utility has been completely redesigned with new options and some old options removed. | Not applicable |  | ext_skel extension tooling. |
| Backward Incompatible Changes | Support for BeOS has been dropped. | Not applicable |  | BeOS support removal. |
| Backward Incompatible Changes | Exceptions thrown due to automatic conversion of warnings into exceptions in EH_THROW mode (e.g. some DateTime exceptions) no longer populate error_get_last() state. | Runtime only |  | Depends on error_get_last() state after exceptions at runtime. |
| Backward Incompatible Changes | TypeError now reports wrong types as `int` and `bool` instead of `integer` and `boolean`. | Not applicable |  | TypeError message wording. |
| Backward Incompatible Changes | Due to the introduction of flexible heredoc/nowdoc syntax (see New Features section), doc strings that contain the ending label inside their body may cause syntax errors or change in interpretation. | Covered | `removed.heredoc_label_in_body` | A heredoc that ends at a line where the label is followed by more words is reported. |
| Backward Incompatible Changes | "continue" statements targeting "switch" control flow structures will now generate a warning. | Covered | `deprecated.continue_switch` | continue; whose innermost target is a switch is reported; on the corpus it matched every warning php -l gives on PHP 7.4. |
| Backward Incompatible Changes | Array accesses of type $obj["123"], where $obj implements ArrayAccess and "123" is an integer string literal will no longer result in an implicit conversion to integer, i.e.,… | Runtime only |  | Depends on $obj being an ArrayAccess object at runtime. |
| Backward Incompatible Changes | In PHP, static properties are shared between inheriting classes, unless the static property is explicitly overridden in a child class. | Gap |  | Test2::$x = &$x reference assignment to an inherited static property is not reported. |
| Backward Incompatible Changes | References returned by array and property accesses are now unwrapped as part of the access. | Runtime only |  | Depends on references existing at runtime. |
| Backward Incompatible Changes | Argument unpacking stopped working with Traversables with non-integer keys. | Runtime only |  | Depends on Traversable keys at runtime. |
| Backward Incompatible Changes | All warnings thrown by BCMath functions are now using PHP's error handling. | Not applicable |  | Warning output channel change. |
| Backward Incompatible Changes | bcmul() and bcpow() now return numbers with the requested scale. | Runtime only |  | Result formatting depends on runtime values. |
| Backward Incompatible Changes | As of PHP 7.3.16, the value of the $childNodes property of DOMDocument, DOMNode, DOMProcessingInstruction, DOMComment, DOMText, DOMCdataSection and DOMNotation is now an empty DOMNodeList instead… | Runtime only |  | DOM childNodes value and IMAP rsh depend on runtime objects and configuration. |
| Backward Incompatible Changes | Due to added support for named captures, mb_ereg_*() patterns using named captures will behave differently. | Covered | `behavior.mb_ereg_named_capture` | mb_ereg_*() with a literal pattern containing a named group is reported. |
| Backward Incompatible Changes | Prepared statements now properly report the fractional seconds for DATETIME/ TIME/TIMESTAMP columns with decimals specifier (e.g. TIMESTAMP(6) when using microseconds). | Runtime only |  | Depends on database column types and returned values. |
| Backward Incompatible Changes | Prepared statements now properly report the fractional seconds for DATETIME/ TIME/TIMESTAMP columns with decimals specifier (e.g. TIMESTAMP(6) when using microseconds). | Runtime only |  | Depends on database column types and PDO emulation mode. |
| Backward Incompatible Changes | Reflection export to string now uses `int` and `bool` instead of `integer` and `boolean`. | Not applicable |  | Reflection string output wording. |
| SAPI | Starting with 7.3.23, incoming cookie names are not url-decoded. | Not applicable |  | SAPI cookie name decoding of incoming requests. |
| SAPI | If an SPL autoloader throws an exception, following autoloaders will not be executed. | Runtime only |  | Depends on autoloaders throwing at runtime. |
| SAPI | Mathematic operations involving SimpleXML objects will now treat the text as an integer or float, whichever is more appropriate. | Runtime only |  | Depends on SimpleXML content at runtime. |
| SAPI | Undefined variables passed to compact() will now be reported as a notice. | Gap |  | compact('undefinedvar') naming a variable never assigned in the scope is not reported. |
| SAPI | getimagesize() and related functions now report the mime type of BMP images as image/bmp instead of image/x-ms-bmp, since the former has been registered with the IANA (see RFC 7903). | Runtime only |  | Depends on image data at runtime. |
| SAPI | stream_socket_get_name() will now return IPv6 addresses wrapped in brackets. | Runtime only |  | Depends on the socket address at runtime. |
| Deprecated Functionality | The declaration of case-insensitive constants has been deprecated. | Covered | `removed.case_insensitive_constant` | define('FOO', 1, true) is reported. |
| Deprecated Functionality | Declaring a function called assert() inside a namespace is deprecated. | Covered | `removed.custom_assert` | A function named assert() declared inside a namespace is reported. |
| Deprecated Functionality | The explicit usage of the constants FILTER_FLAG_SCHEME_REQUIRED and FILTER_FLAG_HOST_REQUIRED is now deprecated; both are implied for FILTER_VALIDATE_URL anyway. | Covered | `removed.filter_flag_scheme_required`<br>`removed.filter_flag_host_required` | FILTER_FLAG_SCHEME_REQUIRED and FILTER_FLAG_HOST_REQUIRED are reported. |
| Deprecated Functionality | image2wbmp() has been deprecated. | Covered | `removed.image2wbmp` | image2wbmp() is reported. |
| Deprecated Functionality | Usage of the Normalizer::NONE form throws a deprecation warning, if PHP is linked with ICU >= 56. | Covered | `removed.builtin_class_constant` | Normalizer::NONE is reported. |
| Deprecated Functionality | The following undocumented mbereg_*() aliases have been deprecated. | Covered | `removed.mbereg`<br>`removed.mbregex_encoding`<br>`removed.mbsplit` | mbereg(), mbregex_encoding(), mbsplit() and mbereg_search_init() are reported. |
| Deprecated Functionality | The pdo_odbc.db2_instance_name ini setting has been formally deprecated. | Covered | `removed.builtin_ini` | pdo_odbc.db2_instance_name is reported in ini files and ini_set(). |
| Deprecated Functionality | Passing a non-string needle to string search functions is deprecated. | Covered | `behavior.int_as_char_code` | strpos($h, 65) is reported. |
| Deprecated Functionality | The fgetss() function and the string.strip_tags stream filter have been deprecated. | Covered | `removed.fgetss`<br>`removed.gzgetss`<br>`removed.builtin_method`<br>`removed.strip_tags_filter` | fgetss(), gzgetss(), SplFileObject::fgetss() and the string.strip_tags stream filter are reported. |
| Changes to INI File Handling | This INI directive has been removed. | Covered | `removed.builtin_ini` | birdstep.max_links is reported in ini files and ini_set(). |
| Changes to INI File Handling | This INI directive has been removed. | Covered | `removed.builtin_ini` | opcache.inherited_hack is reported in php.ini, .user.ini, .htaccess and ini_set(). |

## PHP 7.4

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-7.4/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | Trying to use values of type null, bool, int, float or resource as an array (such as $null["key"]) will now generate a notice. | Runtime only |  | Depends on the runtime type of the array container. |
| Core | get_declared_classes() no longer returns anonymous classes that haven't been instantiated yet. | Runtime only |  | Depends on runtime class instantiation. |
| Core | "fn" is now a reserved keyword. | Covered | `removed.reserved_name` | function fn() {} and class Fn {} are reported. |
| Core | Passing the result of a (non-reference) list() assignment by reference is consistently disallowed now. | Gap |  | foo(list($a) = $b) passing a list() assignment result by reference is not reported. |
| Core | `&lt;?php` at the end of the file (without trailing newline) will now be interpreted as an opening PHP tag. | Covered | `behavior.trailing_open_tag` | A file ending in &lt;?php without a newline is reported. |
| Core | When using include/require on a stream, stream_set_option() will be invoked with the STREAM_OPTION_READ_BUFFER option. | Covered | `behavior.stream_set_option_missing` | stream_wrapper_register() with a project class that has no parent, no traits and no stream_set_option() is reported. |
| Core | The "creating default object from empty value" warning is now consistently thrown if a falsy value is promoted into an stdClass object. | Runtime only |  | Depends on falsy values being promoted at runtime. |
| Core | Previously get_declared_classes() always returned parent classes before child classes. | Runtime only |  | Depends on order of get_declared_classes() at runtime. |
| BCMath | BCMath functions will now warn if a non well-formed number is passed, such as "32foo". | Runtime only |  | Depends on malformed numeric values at runtime. |
| Curl | Attempting to serialize a CURLFile class will now generate an exception. | Runtime only |  | Depends on serializing CURLFile objects at runtime. |
| Curl | Using CURLPIPE_HTTP1 is deprecated, and is no longer supported as of cURL 7.62.0. | Covered | `deprecated.curlpipe_http1` | CURLPIPE_HTTP1 is reported. |
| Curl | The $version parameter of curl_version() is deprecated. | Covered | `removed.builtin_arity` | curl_version(5) with an argument is reported. |
| Date | Calling var_dump() or similar on a DateTime(Immutable) instance will no longer leave behind accessible properties on the object. | Runtime only |  | Depends on var_dump side effects at runtime. |
| Date | Comparison of DateInterval objects (using ==, &lt; and so on) will now generate a warning and always return false. | Runtime only |  | Depends on operand types being DateInterval at runtime. |
| Date | As of PHP 7.4.4, the value of the $childNodes property of DOMDocument, DOMNode, DOMProcessingInstruction, DOMComment, DOMText, DOMCdataSection and DOMNotation is now an empty DOMNodeList instead… | Runtime only |  | Depends on DOM objects at runtime. |
| Intl | The default parameter value of idn_to_ascii() and idn_to_utf8() is now INTL_IDNA_VARIANT_UTS46 instead of the deprecated INTL_IDNA_VARIANT_2003. | Covered | `behavior.idn_default_variant` | idn_to_ascii() and idn_to_utf8() calls that leave the variant to its default are reported. |
| MySQLi | The embedded server functionality has been removed. | Covered | `removed.mysqli_embedded_server` | mysqli_embedded_server_start() and mysqli_embedded_server_end() are reported. |
| MySQLi | The undocumented mysqli::$stat property has been removed in favor of mysqli::stat(). | Covered | `removed.mysqli_stat_property` | $m = new mysqli(); echo $m->stat is reported when the receiver's class is visible in the code. |
| Openssl | The openssl_random_pseudo_bytes() function will now throw an exception in error situations, similar to random_bytes(). | Runtime only |  | Depends on the requested length and entropy at runtime. |
| Pcntl | The $restart_syscalls flag for pcntl_signal() will now be respected for SIGALARM. | Runtime only |  | Signal restart behavior at runtime. |
| PCRE | When PREG_UNMATCHED_AS_NULL mode is used, trailing unmatched capturing groups will now also be set to null (or [null, -1] if offset capture is enabled). | Runtime only |  | Depends on match results at runtime. |
| PEAR | Installation of PEAR (including PECL) is no longer enabled by default. | Not applicable |  | PEAR installation build option. |
| PDO | Attempting to serialize a PDO or PDOStatement instance will now generate an Exception rather than a PDOException, consistent with other internal classes which do not support serialization. | Runtime only |  | Depends on serializing PDO objects at runtime. |
| Reflection | Reflection objects will now generate an exception if an attempt is made to serialize them. | Runtime only |  | Depends on serializing Reflection objects at runtime. |
| Reflection | The signature of the ReflectionMethod::getClosure() method changed to account for existing behavior with static methods: | Not applicable |  | Signature change is LSP compatible. |
| SAPI | Starting with 7.4.11, incoming cookie names are not url-decoded. | Not applicable |  | SAPI cookie name decoding of incoming requests. |
| SPL | Calling get_object_vars() on an ArrayObject instance will now always return the properties of the ArrayObject itself (or a subclass). | Runtime only |  | Depends on ArrayObject instances at runtime. |
| SPL | SplPriorityQueue::setExtractFlags() will throw an exception if zero is passed. | Covered | `removed.extract_flags_zero` | $q = new SplPriorityQueue(); $q->setExtractFlags(0) is reported when the receiver's class is visible in the code. |
| SPL | ArrayObject, ArrayIterator, SplDoublyLinkedList and SplObjectStorage now support the __serialize() + __unserialize() mechanism in addition to the Serializable interface. | Runtime only |  | Serialization payload compatibility across versions at runtime. |
| Standard | The "o" serialization format has been removed. | Runtime only |  | Only affects unserializing crafted strings. |
| Standard | Password hashing algorithm identifiers are now nullable strings rather than integers. | Covered | `behavior.password_algo_integer` | Comparing a PASSWORD_* constant with a number is reported. |
| Standard | htmlentities() will now throw a notice (instead of a strict standards warning) if it is used with an encoding for which only basic entity substitution is supported, in which case it is equivalent… | Runtime only |  | Depends on the encoding value at runtime. |
| Standard | fread() and fwrite() will now return false if the operation failed. | Runtime only |  | Depends on I/O failures at runtime. |
| Standard | fread() and fwrite() on plain files will now throw a notice on failure, such as when trying to write to a read-only file resource. | Runtime only |  | Depends on I/O failures at runtime. |
| Standard | The stream_read() and stream_write() methods on stream wrappers now interpret "false" as a failure return values. | Runtime only |  | Depends on what user stream wrapper methods return at runtime. |
| Standard | round(-0.0) will now return -0.0 rather than +0.0. | Runtime only |  | Depends on values at runtime. |
| Tokenizer | token_get_all() will now emit a T_BAD_CHARACTER token for unexpected characters instead of leaving behind holes in the token stream. | Runtime only |  | Depends on tokenized input at runtime. |
| Core | Nesting ternary operators without explicit parentheses is deprecated: | Covered | `removed.nested_ternary` | $a ? $b : $c ? $d : $e is reported. |
| Core | The array and string offset access syntax using curly braces is deprecated. | Covered | `removed.curly_offset` | $str{0} is reported. |
| Core | The (real) cast is deprecated, use (float) instead. | Covered | `removed.real_cast` | (real) cast is reported. |
| Core | Unbinding $this of a non-static method through a combination of ReflectionMethod::getClosure() and closure rebinding is deprecated. | Runtime only |  | Depends on closure rebinding at runtime. |
| Core | Unbinding $this of a non-static closure that uses $this is deprecated. | Runtime only |  | Depends on closure rebinding at runtime. |
| Core | Using "parent" inside a class without a parent is deprecated, and will throw a compile-time error in the future. | Covered | `removed.parent_without_parent` | parent::f() in a class without a parent is reported. |
| Core | The allow_url_include ini directive is deprecated. | Covered | `deprecated.ini_setting` | allow_url_include=On in php.ini is reported. |
| COM | Importing type libraries with case-insensitive constant registering has been deprecated. | Covered | `deprecated.com_typelib_case_insensitive` | com_load_typelib() with false as the second argument is reported. |
| Filter | FILTER_SANITIZE_MAGIC_QUOTES is deprecated, use FILTER_SANITIZE_ADD_SLASHES instead. | Covered | `removed.filter_sanitize_magic_quotes` | FILTER_SANITIZE_MAGIC_QUOTES is reported. |
| Mbstring | Passing a non-string pattern to mb_ereg_replace() is deprecated. | Covered | `behavior.int_as_char_code` | mb_ereg_replace(65, 'b', $s) is reported. |
| Mbstring | Passing the encoding as 3rd parameter to mb_strrpos() is deprecated. | Covered | `removed.mb_strrpos_encoding` | mb_strrpos($h, $n, 'UTF-8') is reported. |
| LDAP | ldap_control_paged_result_response and ldap_control_paged_result are deprecated. | Covered | `removed.ldap_control_paged_result`<br>`removed.ldap_control_paged_result_response` | ldap_control_paged_result() and ldap_control_paged_result_response() are reported. |
| Reflection | Calls to ReflectionType::__toString() now generate a deprecation notice. | Not applicable |  | Only PHP 7.4 raises this deprecation; it was withdrawn in 8.0, so no upgrade target past 7.4 sees it. |
| Reflection | The export() methods on all Reflection classes are deprecated. | Covered | `removed.reflection_export` | ReflectionClass::export() and ReflectionFunction::export() are reported. |
| Socket | The AI_IDN_ALLOW_UNASSIGNED and AI_IDN_USE_STD3_ASCII_RULES flags for socket_addrinfo_lookup() are deprecated, due to an upstream deprecation in glibc. | Covered | `removed.ai_idn_constant` | AI_IDN_ALLOW_UNASSIGNED and AI_IDN_USE_STD3_ASCII_RULES are reported. |
| Standard | Passing invalid characters to ''base_convert()'', ''bindec()'', ''octdec()'' and ''hexdec()'' will now generate a deprecation notice. | Partial | `deprecated.invalid_digits` | bindec(), hexdec() and octdec() with literal invalid digits are reported; base_convert() is not. |
| Standard | Using array_key_exists() on objects is deprecated. | Covered | `removed.array_key_exists_object` | array_key_exists() on $this or a new object is reported; other objects need the argument type. |
| Standard | The is_real() function is deprecated, use is_float() instead. | Covered | `removed.is_real` | is_real() is reported. |
| Standard | The get_magic_quotes_gpc() and get_magic_quotes_runtime() functions are deprecated. | Covered | `removed.get_magic_quotes_gpc`<br>`removed.get_magic_quotes_runtime` | get_magic_quotes_gpc() and get_magic_quotes_runtime() are reported. |
| Standard | The hebrevc() function is deprecated. | Covered | `removed.hebrevc` | hebrevc() is reported. |
| Standard | The convert_cyr_string() function is deprecated. | Covered | `removed.convert_cyr_string` | convert_cyr_string() is reported. |
| Standard | The money_format() function is deprecated. | Covered | `removed.money_format` | money_format() is reported. |
| Standard | The ezmlm_hash() function is deprecated. | Covered | `removed.ezmlm_hash` | ezmlm_hash() is reported. |
| Standard | The restore_include_path() function is deprecated. | Covered | `removed.restore_include_path` | restore_include_path() is reported. |
| Standard | Passing parameters to implode() in reverse order is deprecated, use implode($glue, $parts) instead of implode($parts, $glue). | Covered | `removed.implode_order` | implode($parts, ', ') with reversed arguments is reported. |
| Interbase | The interbase extension has been moved to PECL. | Covered | `removed.interbase` | ibase_connect() is reported. |
| Recode | The recode extension has been moved to PECL. | Covered | `removed.recode` | recode_string() is reported. |
| WDDX | The WDDX extension has been deprecated and moved to PECL. | Covered | `removed.wddx` | wddx_serialize_value() is reported. |

## PHP 8.0

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-8.0/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | `match` is now a reserved keyword. | Covered | `removed.reserved_name` | function match() and class Match are reported; a bare call to an externally defined match() function is not. |
| Core | Assertion failures now throw by default. | Runtime only |  | Whether an assert() fails depends on runtime values; assert.exception default change is not visible in source. |
| Core | Methods with the same name as the class are no longer interpreted as constructors. | Covered | `removed.php4_constructor` | class Foo { function Foo() {} } is reported. |
| Core | Removed ability to call non-static methods statically. | Partial | `removed.static_call_to_instance_method` | A::f() on a non-static method is reported, but is_callable(['A', 'f']) / is_callable('A::f') checks are not. |
| Core | Removed (real) cast. | Covered | `removed.real_cast` | (real) $y is reported. |
| Core | Removed (unset) cast. | Covered | `removed.unset_cast` | (unset) $y is reported. |
| Core | Removed track_errors ini directive. | Covered | `removed.magic_variable`<br>`removed.ini_setting`<br>`deprecated.ini_setting` | $php_errormsg, ini_set('track_errors') and track_errors in php.ini/.htaccess are reported. |
| Core | Removed the ability to define case-insensitive constants. | Covered | `removed.case_insensitive_constant` | define('FOO', 1, true) is reported. |
| Core | Access to undefined constants now always results in an Error exception. | Partial | `removed.bareword_key` | Unquoted array keys ($a[foo]) are reported, but bareword constants elsewhere ($x = bar;) are not. |
| Core | Removed ability to specify an autoloader using an __autoload() function. | Covered | `removed.__autoload` | function __autoload() is reported. |
| Core | Removed the $errcontext argument for custom error handlers. | Covered | `removed.error_handler_context` | set_error_handler with a five-parameter closure is reported. |
| Core | Removed create_function(). | Covered | `removed.create_function` | create_function() is reported. |
| Core | Removed each(). foreach or ArrayIterator should be used instead. | Covered | `removed.each` | each() is reported. |
| Core | Removed ability to unbind $this from closures that were created from a method, using Closure::fromCallable() or ReflectionMethod::getClosure(). | Runtime only |  | Unbinding $this from a fromCallable/getClosure closure depends on what the closure variable holds at runtime. |
| Core | Also removed ability to unbind $this from proper closures that contain uses of $this. | Runtime only |  | bindTo(null)/Closure::bind(..., null) on a closure using $this was tested and not reported; the closure is normally passed via variables, so it is runtime. |
| Core | Removed ability to use array_key_exists() with objects. | Runtime only |  | Whether array_key_exists() receives an object depends on the runtime type of its argument. |
| Core | Made the behavior of array_key_exists() regarding the type of the key parameter consistent with isset() and normal array access. | Runtime only |  | Key type coercion in array_key_exists() depends on runtime key values. |
| Core | Any array that has a number n as its first numeric key will use n+1 for its next implicit key, even if n is negative. | Covered | `behavior.negative_array_key` | Array literals with a negative first key followed by an implicit key, and array_fill() with a negative start, are reported. |
| Core | The default error_reporting level is now E_ALL. | Not applicable |  | Change of the default error_reporting level; not expressed in application source. |
| Core | display_startup_errors is now enabled by default. | Not applicable |  | Change of the display_startup_errors default; not expressed in application source. |
| Core | Using "parent" inside a class that has no parent will now result in a fatal compile-time error. | Covered | `removed.parent_without_parent` | parent::f() in a class without a parent is reported. |
| Core | The @ operator will no longer silence fatal errors (E_ERROR, E_CORE_ERROR, E_COMPILE_ERROR, E_USER_ERROR, E_RECOVERABLE_ERROR, E_PARSE). | Covered | `behavior.error_reporting_zero` | An error handler testing error_reporting() == 0 is reported. |
| Core | Following the hash comment operator # immediately with an opening bracket is not supported as a comment anymore since this syntax is now used for attributes. | Covered | `removed.hash_bracket_comment` | A '#[' comment is reported. |
| Core | Inheritance errors due to incompatible method signatures (LSP violations) will now always generate a fatal error. | Covered | `removed.incompatible_user_override` | A child method with an incompatible signature to its user-defined parent is reported. |
| Core | The precedence of the concatenation operator has changed relative to bitshifts and addition as well as subtraction. | Covered | `behavior.concat_precedence` | echo "Sum: " . $a + $b; is reported. |
| Core | Arguments with a default value that resolves to null at runtime will no longer implicitly mark the argument type as nullable. | Covered | `removed.null_constant_default` | A typed, non-nullable parameter whose default is a constant declared null in the same file is reported. |
| Core | A number of warnings have been converted into Error exceptions: | Runtime only |  | Writing properties/indexes on non-objects/scalars etc. depends on runtime value types. |
| Core | Attempting to assign multiple bytes to a string offset will now emit a warning. | Runtime only |  | Multi-byte assignment to a string offset depends on runtime string values. |
| Core | Unexpected characters in source files (such as null bytes outside of strings) will now result in a ParseError exception instead of a compile warning. | Covered | `removed.nul_byte_in_source` | A NUL byte in code outside strings and comments is reported. |
| Core | Uncaught exceptions now go through "clean shutdown", which means that destructors will be called after an uncaught exception. | Runtime only |  | Destructor execution after uncaught exceptions is runtime behavior. |
| Core | Compile time fatal error "Only variables can be passed by reference" has been delayed until runtime and converted to "Argument cannot be passed by reference" exception. | Not applicable |  | The code was already a compile-time fatal error in PHP 7, so working code cannot contain it. |
| Core | Some "Only variables should be passed by reference" notices have been converted to "Argument cannot be passed by reference" exception. | Runtime only |  | Which by-reference notices became exceptions depends on runtime call targets; end(explode()) and &array_pop() were not reported. |
| Core | The generated name for anonymous classes has changed. | Runtime only |  | Only code comparing generated anonymous class names at runtime is affected. |
| Core | Non-absolute trait method references in trait alias adaptations are now required to be unambiguous: | Covered | `removed.ambiguous_trait_alias` | An unqualified alias for a method that more than one of the used project traits defines is reported. |
| Core | The signature of abstract methods defined in traits is now checked against the implementing class method: | Gap |  | Not reported: a class implementing a trait's abstract method with an incompatible return type (string vs int). |
| Core | Disabled functions are now treated exactly like non-existent functions. | Runtime only |  | Disabled functions depend on the server's disable_functions configuration at runtime. |
| Core | data: stream wrappers are no longer writable, which matches the documented behavior. | Covered | `removed.data_stream_write` | file_put_contents() and fopen() in a writing mode on a literal data: URL are reported. |
| Core | The arithmetic and bitwise operators +, -, *, /, **, %, &lt;&lt;, >>, &, \|, ^, ~, ++, -- will now consistently throw a TypeError when one of the operands is an array, resource or non-overloaded object. | Runtime only |  | Arithmetic on arrays/resources/objects depends on runtime operand types. |
| Core | Float to string casting will now always behave locale-independently. | Runtime only |  | Float-to-string output differs only when a non-C LC_NUMERIC locale is active at runtime. |
| Core | Removed support for deprecated curly braces for offset access RFC: | Covered | `removed.curly_offset` | $s{0} is reported. |
| Core | Applying the final modifier on a private method will now produce a warning unless that method is the constructor. | Covered | `deprecated.final_private_method` | class A { final private function f() {} } is reported. |
| Core | If an object constructor exit()s, the object destructor will no longer be called. | Runtime only |  | Destructor behavior after exit() in a constructor is runtime behavior. |
| Core | Non-strict comparisons between numbers and non-numeric strings now work by casting the number to string and comparing the strings. | Runtime only |  | Number vs non-numeric string comparison results depend on runtime values. |
| Core | Namespaced names can no longer contain whitespace: | Covered | `removed.namespaced_name_whitespace` | Names written as Foo \ Bar, with whitespace around the separator, are reported. |
| Core | Nested ternaries now require explicit parentheses. | Covered | `removed.nested_ternary` | $a ? $b : $c ? $d : $e is reported. |
| Core | debug_backtrace() and Exception::getTrace() will no longer provide references to arguments. | Runtime only |  | Only code modifying arguments through debug_backtrace() results at runtime is affected. |
| Core | Numeric string handling has been altered to be more intuitive and less error-prone. | Runtime only |  | Numeric string handling depends on runtime string values. |
| Core | Magic Methods will now have their arguments and return types checked if they have them declared. | Covered | `removed.magic_method_signature` | Magic methods whose declared parameter or return types don't match what PHP expects, such as __get(int $name) or __clone(): int, are reported. |
| Core | call_user_func_array() array keys will now be interpreted as parameter names, instead of being silently ignored. | Covered | `behavior.call_user_func_array_named` | call_user_func_array() with a literal array that has string keys is reported. |
| COM | Removed the ability to import case-insensitive constants from type libraries. | Covered | `deprecated.com_typelib_case_insensitive`<br>`removed.builtin_ini` | com_load_typelib(..., false) and com.autoregister_casesensitive are reported. |
| Curl | CURLOPT_POSTFIELDS no longer accepts objects as arrays. | Runtime only |  | Whether CURLOPT_POSTFIELDS receives an object depends on the runtime value. |
| Date | mktime() and gmmktime() now require at least one argument. | Covered | `removed.builtin_arity` | mktime() and gmmktime() with no arguments are reported. |
| dom | Remove unimplemented classes from ext/dom that had no behavior and contained test data. | Covered | `removed.builtin_class` | new DOMConfiguration() is reported; type hints/instanceof on these classes do not fail at runtime. |
| Enchant | enchant_broker_list_dicts(), enchant_broker_describe() and enchant_dict_suggest() will now return an empty array instead of null. | Runtime only |  | Return value change (empty array instead of null) only matters for runtime checks on the result. |
| Enchant | enchant_broker_init() will now return an EnchantBroker object rather than a resource. | Covered | `removed.resource_check` | is_resource() on enchant_broker_init() result is reported. |
| Enchant | enchant_broker_request_dict() and enchant_broker_request_pwl_dict() will now return an EnchantDictionary object rather than a resource. | Covered | `removed.resource_check` | is_resource() on enchant_broker_request_dict() and enchant_broker_request_pwl_dict() is reported. |
| Exif | Removed read_exif_data(). | Covered | `removed.read_exif_data` | read_exif_data() is reported. |
| Filter | The FILTER_FLAG_SCHEME_REQUIRED and FILTER_FLAG_HOST_REQUIRED flags for the FILTER_VALIDATE_URL filter have been removed. | Covered | `removed.filter_flag_scheme_required`<br>`removed.filter_flag_host_required` | Both FILTER_FLAG_*_REQUIRED constants are reported. |
| Filter | The INPUT_REQUEST and INPUT_SESSION source for filter_input() etc have been removed. | Covered | `removed.input_request`<br>`removed.input_session` | INPUT_REQUEST and INPUT_SESSION are reported. |
| GD | The GD extension now uses a GdImage objects as the underlying data structure for images, rather than resources. | Covered | `removed.resource_check` | is_resource() on an imagecreatetruecolor() result is reported. |
| GD | The deprecated function image2wbmp() has been removed. | Covered | `removed.image2wbmp` | image2wbmp() is reported. |
| GD | The deprecated functions png2wbmp() and jpeg2wbmp() have been removed. | Covered | `removed.png2wbmp`<br>`removed.jpeg2wbmp` | png2wbmp() and jpeg2wbmp() are reported. |
| GD | The default $mode parameter of imagecropauto() no longer accepts -1. | Covered | `removed.imagecropauto_default` | imagecropauto($im, -1) is reported. |
| GMP | gmp_random() has been removed. | Covered | `removed.gmp_random` | gmp_random() is reported. |
| Iconv | iconv() implementations which do not properly set errno in case of errors are no longer supported. | Not applicable |  | Concerns the iconv implementation PHP is built against. |
| IMAP | The unused default_host argument of imap_headerinfo() has been removed. | Covered | `removed.imap` | imap_headerinfo() with five arguments is reported, via the ext/imap unbundling rule rather than the argument removal. |
| IMAP | The imap_header() function which is an alias of imap_headerinfo() has been removed. | Covered | `removed.imap` | imap_header() is reported via the ext/imap unbundling rule. |
| Intl | The deprecated constant INTL_IDNA_VARIANT_2003 has been removed. | Covered | `removed.intl_idna_variant_2003` | INTL_IDNA_VARIANT_2003 is reported. |
| Intl | The deprecated Normalizer::NONE constant has been removed. | Covered | `removed.builtin_class_constant` | Normalizer::NONE is reported. |
| Intl | The IntlDateFormatter::RELATIVE_FULL, IntlDateFormatter::RELATIVE_LONG, IntlDateFormatter::RELATIVE_MEDIUM, and IntlDateFormatter::RELATIVE_SHORT constants have been added. | Not applicable |  | New constants were added; existing code is unaffected. |
| LDAP | The deprecated function ldap_sort has been removed. | Covered | `removed.ldap_sort` | ldap_sort() is reported. |
| LDAP | The deprecated function ldap_control_paged_result has been removed. | Covered | `removed.ldap_control_paged_result` | ldap_control_paged_result() is reported. |
| LDAP | The deprecated function ldap_control_paged_result_response has been removed. | Covered | `removed.ldap_control_paged_result_response` | ldap_control_paged_result_response() is reported. |
| LDAP | The interface of ldap_set_rebind_proc has changed; the $callback parameter does not accept empty string anymore; null value shall be used instead. | Covered | `deprecated.ldap_rebind_empty` | ldap_set_rebind_proc($l, '') is reported. |
| Mbstring | The mbstring.func_overload directive has been removed. | Covered | `removed.mb_overload`<br>`deprecated.ini_setting` | MB_OVERLOAD_* constants, ini_set and php.ini/.htaccess mbstring.func_overload are reported; mb_get_info('func_overload') is not. |
| Mbstring | mb_parse_str() can no longer be used without specifying a result array. | Covered | `removed.builtin_arity` | mb_parse_str($q) with one argument is reported. |
| Mbstring | A number of deprecated mbregex aliases have been removed. | Covered | `removed.mbregex_encoding`<br>`removed.mbereg`<br>`removed.mbsplit` | mbregex_encoding(), mbereg(), mberegi(), mbereg_replace() and mbsplit() are reported. |
| Mbstring | The 'e' modifier for mb_ereg_replace() has been removed. | Covered | `removed.mb_ereg_e_option` | mb_ereg_replace('a', $c, $s, 'e') is reported. |
| Mbstring | A non-string pattern argument to mb_ereg_replace() will now be interpreted as a string instead of an ASCII codepoint. | Covered | `behavior.int_as_char_code` | mb_ereg_replace(65, 'b', $s) is reported. |
| Mbstring | The needle argument for mb_strpos(), mb_strrpos(), mb_stripos(), mb_strripos(), mb_strstr(), mb_stristr(), mb_strrchr() and mb_strrichr() can now be empty. | Runtime only |  | Return value for an empty needle changes; the needle is normally a variable. |
| Mbstring | The $is_hex parameter, which was not used internally, has been removed from mb_decode_numericentity(). | Covered | `removed.builtin_arity` | mb_decode_numericentity() with a fourth $is_hex argument is reported. |
| Mbstring | The legacy behavior of passing the encoding as the third argument instead of an offset for the mb_strrpos() function has been removed; provide an explicit 0 offset with the encoding as the fourth… | Covered | `removed.mb_strrpos_encoding` | mb_strrpos($h, $n, 'UTF-8') is reported. |
| Mbstring | The ISO_8859-* character encoding aliases have been replaced by ISO8859-* aliases for better interoperability with the iconv extension. | Covered | `removed.mbstring_iso_alias` | ISO_8859-* and ISO8859_* names passed to mb_*() functions are reported. |
| Mbstring | mb_ereg() and mb_eregi() will now return boolean true on a successful match. | Runtime only |  | Return value change of mb_ereg() only matters for runtime use of the result. |
| OCI8 | The OCI-Lob class is now called OCILob, and the OCI-Collection class is now called OCICollection for name compliance enforced by PHP 8 arginfo type annotation tooling. | Covered | `behavior.oci_class_name` | The literal class names 'OCI-Lob' and 'OCI-Collection' are reported. |
| OCI8 | Several alias functions have been marked as deprecated. | Covered | `deprecated.oci8_alias` | The OCI8 aliases without the oci_ prefix, such as ocilogon() and ociparse(), are reported. |
| OCI8 | oci_internal_debug() and its alias ociinternaldebug() have been removed. | Covered | `deprecated.oci8_alias`<br>`removed.oci8` | oci_internal_debug() and its alias ociinternaldebug() are reported. |
| ODBC | odbc_connect() no longer reuses persistent connections. | Not applicable |  | Connection pooling behavior of odbc_connect(); no source change required. |
| ODBC | The unused flags parameter of odbc_exec() has been removed. | Covered | `removed.builtin_arity` | odbc_exec($c, $sql, 1) with the flags argument is reported. |
| OpenSSL | openssl_x509_read() and openssl_csr_sign() will now return an OpenSSLCertificate object rather than a resource. | Covered | `removed.resource_check` | is_resource() on openssl_x509_read() and openssl_csr_sign() is reported. |
| OpenSSL | The openssl_x509_free() function is deprecated and no longer has an effect, instead the OpenSSLCertificate instance is automatically destroyed if it is no longer referenced. | Covered | `deprecated.openssl_x509_free` | openssl_x509_free() is reported. |
| OpenSSL | openssl_csr_new() will now return an OpenSSLCertificateSigningRequest object rather than a resource. | Covered | `removed.resource_check` | is_resource() on openssl_csr_new() is reported. |
| OpenSSL | openssl_pkey_new() will now return an OpenSSLAsymmetricKey object rather than a resource. | Covered | `removed.resource_check` | is_resource() on openssl_pkey_new() is reported. |
| OpenSSL | The openssl_pkey_free() function is deprecated and no longer has an effect, instead the OpenSSLAsymmetricKey instance is automatically destroyed if it is no longer referenced. | Covered | `deprecated.openssl_pkey_free` | openssl_pkey_free() is reported. |
| OpenSSL | openssl_seal() and openssl_open() now require $method to be passed, as the previous default of "RC4" is considered insecure. | Covered | `removed.builtin_arity` | openssl_seal() and openssl_open() with four arguments (no method) are reported. |
| PCRE | When passing invalid escape sequences they are no longer interpreted as literals. | Covered | `removed.regex_invalid_escape` | Literal preg_*() patterns with the X modifier or an unknown letter escape such as \i are reported; PHP 8.0 rejects them. |
| PDO | The default error handling mode has been changed from "silent" to "exceptions". | Runtime only |  | Whether code relied on silent PDO errors depends on runtime query failures and error handling flow. |
| PDO | The signatures of some PDO methods have changed: | Covered | `removed.incompatible_override` | Subclasses overriding PDO::query() and PDOStatement::setFetchMode() with old signatures are reported. |
| PDO MySQL | PDO::inTransaction() now reports the actual transaction state of the connection, rather than an approximation maintained by PDO. | Runtime only |  | inTransaction() result depends on runtime transaction state. |
| PDO_ODBC | The php.ini directive pdo_odbc.db2_instance_name has been removed | Covered | `removed.builtin_ini` | pdo_odbc.db2_instance_name is reported in ini files and ini_set(). |
| pgsql | The deprecated pg_connect() syntax using multiple parameters instead of a connection string is no longer supported. | Covered | `removed.builtin_arity` | pg_connect() with five separate parameters is reported. |
| pgsql | The deprecated pg_lo_import() and pg_lo_export() signature that passes the connection as the last argument is no longer supported. | Runtime only |  | Old vs new pg_lo_import()/pg_lo_export() argument order is indistinguishable when both arguments are variables. |
| pgsql | pg_fetch_all() will now return an empty array instead of false for result sets with zero rows. | Runtime only |  | pg_fetch_all() result checks against false depend on runtime result sets. |
| Phar | Metadata associated with a phar will no longer be automatically unserialized, to fix potential security vulnerabilities due to object instantiation, autoloading, etc. RFC: | Runtime only |  | Phar metadata unserialization is runtime behavior. |
| Reflection | The method signatures ReflectionClass::newInstance($args) ReflectionFunction::invoke($args) ReflectionMethod::invoke($object, $args) have been changed to: | Covered | `removed.incompatible_override` | Subclasses overriding ReflectionClass::newInstance()/ReflectionMethod::invoke() with old signatures are reported. |
| Reflection | The ReflectionType::__toString() method will now return a complete debug representation of the type, and is no longer deprecated. | Runtime only |  | Only code parsing ReflectionType::__toString() output at runtime is affected. |
| Reflection | Reflection export() methods have been removed. | Covered | `removed.reflection_export` | ReflectionClass::export() and Reflection::export() are reported. |
| Reflection | The following methods can now return information about default values of parameters of internal functions: | Not applicable |  | New capability for internal functions; existing code is unaffected. |
| Reflection | ReflectionMethod::isConstructor() and ReflectionMethod::isDestructor() now also return true for `__construct` and `__destruct` methods of interfaces. | Runtime only |  | Return value change of isConstructor()/isDestructor() depends on the reflected method at runtime. |
| Reflection | ReflectionType::isBuiltin() method has been moved to ReflectionNamedType. | Runtime only |  | Whether isBuiltin() is called on a ReflectionUnionType depends on the runtime type object. |
| Sockets | The deprecated AI_IDN_ALLOW_UNASSIGNED and AI_IDN_USE_STD3_ASCII_RULES flags for socket_addrinfo_lookup() have been removed. | Covered | `removed.ai_idn_constant` | AI_IDN_ALLOW_UNASSIGNED and AI_IDN_USE_STD3_ASCII_RULES are reported. |
| Sockets | socket_create(), socket_create_listen(), socket_accept(), socket_import_stream(), socket_addrinfo_connect(), socket_addrinfo_bind(), and socket_wsaprotocol_info_import() will now return a Socket… | Covered | `removed.resource_check` | is_resource() on every listed socket function is reported. |
| Sockets | socket_addrinfo_lookup() will now return an array of AddressInfo objects rather than resources. | Runtime only |  | is_resource() checks on socket_addrinfo_lookup() array elements depend on data flow through arrays. |
| SPL | SplFileObject::fgetss() has been removed. | Covered | `removed.builtin_method` | SplFileObject::fgetss() on a new SplFileObject is reported. |
| SPL | SplFileObject::seek() now always seeks to the beginning of the line. | Runtime only |  | seek() position change depends on runtime file content. |
| SPL | SplHeap::compare($a, $b) now specifies a method signature. | Covered | `removed.incompatible_override`<br>`deprecated.tentative_return_type` | A SplHeap subclass with an incompatible compare() signature is reported. |
| SPL | SplDoublyLinkedList::push() now returns void instead of true | Runtime only |  | Only code using the return value of push() is affected, which needs the receiver type. |
| SPL | SplDoublyLinkedList::unshift() now returns void instead of true | Runtime only |  | Only code using the return value of unshift() is affected, which needs the receiver type. |
| SPL | SplQueue::enqueue() now returns void instead of true | Runtime only |  | Only code using the return value of enqueue() is affected, which needs the receiver type. |
| SPL | spl_autoload_register() will now always throw a TypeError on invalid arguments, therefore the second argument $do_throw is ignored and a notice will be emitted if it is set to false. | Covered | `behavior.spl_autoload_register_throw` | spl_autoload_register('loader', false) is reported. |
| SPL | SplFixedArray is now an IteratorAggregate and not an Iterator. | Covered | `removed.builtin_method` | rewind()/valid()/current()/next() on a SplFixedArray are reported. |
| Standard | assert() will no longer evaluate string arguments, instead they will be treated like any other argument. | Covered | `removed.assert_string`<br>`removed.assert_quiet_eval`<br>`removed.builtin_ini` | assert(...), ASSERT_QUIET_EVAL and the assert.quiet_eval setting are reported. |
| Standard | parse_str() can no longer be used without specifying a result array. | Covered | `removed.parse_str_one_arg`<br>`removed.builtin_arity` | parse_str($q) with one argument is reported. |
| Standard | fgetss() has been removed. | Covered | `removed.fgetss` | fgetss() is reported. |
| Standard | The string.strip_tags filter has been removed. | Covered | `removed.strip_tags_filter` | stream_filter_append($fp, 'string.strip_tags') is reported. |
| Standard | The needle argument of strpos(), strrpos(), stripos(), strripos(), strstr(), strchr(), strrchr(), and stristr() will now always be interpreted as a string. | Covered | `behavior.int_as_char_code` | strstr($s, 10) is reported. |
| Standard | The needle argument for strpos(), strrpos(), stripos(), strripos(), strstr(), stristr() and strrchr() can now be empty. | Runtime only |  | Return value for an empty needle changes; the needle is normally a variable. |
| Standard | The length argument for substr(), substr_count(), substr_compare(), and iconv_substr() can now be null. | Covered | `behavior.null_length` | substr($s, 2, null) is reported. |
| Standard | The length argument for array_splice() can now be null. | Covered | `behavior.null_length` | array_splice($a, 1, null) is reported. |
| Standard | The args argument of vsprintf(), vfprintf(), and vprintf() must now be an array. | Runtime only |  | The args argument of vsprintf() is normally a variable whose type is known only at runtime. |
| Standard | The 'salt' option of password_hash() is no longer supported. | Covered | `behavior.password_hash_salt` | password_hash($p, PASSWORD_DEFAULT, ['salt' => $salt]) is reported. |
| Standard | The quotemeta() function will now return an empty string if an empty string was passed. | Runtime only |  | Return value for an empty input depends on runtime values. |
| Standard | hebrevc() has been removed. | Covered | `removed.hebrevc` | hebrevc() is reported. |
| Standard | convert_cyr_string() has been removed. | Covered | `removed.convert_cyr_string` | convert_cyr_string() is reported. |
| Standard | money_format() has been removed. | Covered | `removed.money_format` | money_format() is reported. |
| Standard | ezmlm_hash() has been removed. | Covered | `removed.ezmlm_hash` | ezmlm_hash() is reported. |
| Standard | restore_include_path() has been removed. | Covered | `removed.restore_include_path` | restore_include_path() is reported. |
| Standard | get_magic_quotes_gpc() and get_magic_quotes_runtime() has been removed. | Covered | `removed.get_magic_quotes_gpc`<br>`removed.get_magic_quotes_runtime` | get_magic_quotes_gpc() and get_magic_quotes_runtime() are reported. |
| Standard | FILTER_SANITIZE_MAGIC_QUOTES has been removed. | Covered | `removed.filter_sanitize_magic_quotes` | FILTER_SANITIZE_MAGIC_QUOTES is reported. |
| Standard | Calling implode() with parameters in a reverse order ($pieces, $glue) is no longer supported. | Covered | `removed.implode_order` | implode($pieces, ', ') with a known array first is reported. |
| Standard | parse_url() will now distinguish absent and empty queries and fragments: | Runtime only |  | Null vs empty query/fragment depends on runtime URLs. |
| Standard | var_dump() and debug_zval_dump() will now print floating-point numbers using serialize_precision rather than precision. | Not applicable |  | Debug output formatting of var_dump()/debug_zval_dump(). |
| Standard | If the array returned by __sleep() contains non-existing properties, these are now silently ignored. | Runtime only |  | Depends on which properties exist at runtime when __sleep() runs. |
| Standard | The default locale on startup is now always "C". | Runtime only |  | Locale-dependent behavior depends on the runtime environment. |
| Standard | Removed deprecated DES fallback in crypt(). | Runtime only |  | The crypt() salt format is normally a runtime value. |
| Standard | Specifying out of range rounds for sha256/sha512 crypt() will now fail with *0 instead of clamping to the closest limit. | Runtime only |  | The crypt() rounds value is normally a runtime value. |
| Standard | The result of sorting functions may have changed, if the array contains elements that compare as equal. | Runtime only |  | Sort result order for equal elements depends on runtime data. |
| Standard | Sort comparison functions that return true or false will now throw a deprecation warning, and should be replaced with an implementation that returns an integer less than, equal to, or greater than… | Covered | `deprecated.bool_sort_comparator` | usort(), uasort() and uksort() with a closure that only returns a comparison are reported. |
| Standard | Any functions accepting callbacks that are not explicitly specified to accept parameters by reference will now warn if a callback with reference parameters is used. | Covered | `behavior.reference_callback` | Closures with a by-reference parameter passed to array_filter(), array_map(), array_reduce(), preg_replace_callback(), usort(), uasort() or uksort() are reported. |
| Standard | The HTTP stream wrapper as used by functions like file_get_contents() now advertises HTTP/1.1 rather than HTTP/1.0 by default. | Not applicable |  | HTTP protocol version advertised by the stream wrapper; no source change required. |
| Standard | Calling crypt() without an explicit salt is no longer supported. | Covered | `removed.builtin_arity` | crypt($pw) without a salt is reported. |
| Standard | substr(), mb_substr(), iconv_substr() and grapheme_substr() now consistently clamp out-of-bounds offsets to the string boundary. | Runtime only |  | Out-of-bounds offsets depend on runtime string lengths. |
| Standard | Populating $http_response_header variable by the HTTP stream wrapper doesn't force rebuilding of symbol table anymore. | Not applicable |  | Internal symbol table handling. |
| Sysvmsg | msg_get_queue() will now return an SysvMessageQueue object rather than a resource. | Covered | `removed.resource_check` | is_resource() on msg_get_queue() is reported. |
| Sysvsem | sem_get() will now return an SysvSemaphore object rather than a resource. | Covered | `removed.resource_check` | is_resource() on sem_get() is reported. |
| Sysvsem | The $auto_release parameter of sem_get() was changed to accept bool values rather than int. | Runtime only |  | An int auto_release is still coerced to bool outside strict_types; only typed runtime values in strict mode fail. |
| Sysvshm | shm_attach() will now return an SysvSharedMemory object rather than a resource. | Covered | `removed.resource_check` | is_resource() on shm_attach() is reported. |
| tidy | The $use_include_path parameter, which was not used internally, has been removed from tidy_repair_string(). | Covered | `removed.builtin_arity` | tidy_repair_string() with a fourth $use_include_path argument is reported. |
| tidy | tidy::repairString() and tidy::repairFile() became static methods | Covered | `removed.builtin_override_signature` | Non-static overrides of tidy::repairString() and tidy::repairFile() are reported. |
| Tokenizer | T_COMMENT tokens will no longer include a trailing newline. | Runtime only |  | Only code inspecting token_get_all() output at runtime is affected. |
| Tokenizer | Namespaced names are now represented using the T_NAME_QUALIFIED (Foo\Bar), T_NAME_FULLY_QUALIFIED (\Foo\Bar) and T_NAME_RELATIVE (namespace\Foo\Bar) tokens. | Runtime only |  | Only code inspecting token_get_all() output at runtime is affected. |
| XML | xml_parser_create(_ns) will now return an XMLParser object rather than a resource. | Covered | `removed.resource_check`<br>`deprecated.xml_parser_free` | is_resource() on xml_parser_create() and xml_parser_free() are reported. |
| XMLReader | XMLReader::open() and XMLReader::xml() are now static methods. | Covered | `removed.incompatible_override` | A subclass overriding XMLReader::open()/xml() as non-static is reported. |
| XMLWriter | The XMLWriter functions now accept and return, respectively, XMLWriter objects instead of resources. | Covered | `removed.resource_check` | is_resource() on xmlwriter_open_memory() is reported. |
| Zip | ZipArchive::OPSYS_Z_CPM has been removed (this name was a typo). | Covered | `removed.builtin_class_constant` | ZipArchive::OPSYS_Z_CPM is reported. |
| Zlib | gzgetss() has been removed. | Covered | `removed.gzgetss` | gzgetss() is reported. |
| Zlib | inflate_init() will now return an InflateContext object rather than a resource. | Covered | `removed.resource_check` | is_resource() on inflate_init() is reported. |
| Zlib | deflate_init() will now return a DeflateContext object rather than a resource. | Covered | `removed.resource_check` | is_resource() on deflate_init() is reported. |
| Zlib | zlib.output_compression is no longer automatically disabled for Content-Type: | Not applicable |  | Output compression behavior for image content types; server-level runtime behavior. |
| Core | Declaring a required parameter after an optional one is deprecated. | Covered | `deprecated.optional_before_required` | function f($a = 1, $b) {} is reported. |
| Core | Calling get_defined_functions() with $exclude_disabled explicitly set to false is deprecated. | Covered | `deprecated.get_defined_functions_arg` | get_defined_functions(false) is reported. |
| Enchant | enchant_broker_set_dict_path and enchant_broker_get_dict_path not available in libenchant &lt; 1.5 nor in libenchant-2 | Covered | `deprecated.builtin_function` | enchant_broker_set_dict_path() and enchant_broker_get_dict_path() are reported. |
| Enchant | enchant_dict_add_to_personal, use enchant_dict_add instead | Covered | `deprecated.builtin_function` | enchant_dict_add_to_personal() is reported. |
| Enchant | enchant_dict_is_in_session, use enchant_dict_is_added instead | Covered | `deprecated.builtin_function` | enchant_dict_is_in_session() is reported. |
| Enchant | enchant_broker_free and enchant_broker_free_dict, unset the object instead | Covered | `deprecated.builtin_function` | enchant_broker_free() and enchant_broker_free_dict() are reported. |
| Enchant | ENCHANT_MYSPELL and ENCHANT_ISPELL constants | Covered | `deprecated.builtin_constant` | ENCHANT_MYSPELL and ENCHANT_ISPELL are reported. |
| LibXML | libxml_disable_entity_loader() has been deprecated. | Covered | `deprecated.libxml_disable_entity_loader` | libxml_disable_entity_loader() is reported. |
| PGSQL / PDO PGSQL | The constant PGSQL_LIBPQ_VERSION_STR has now the same value as PGSQL_LIBPQ_VERSION, and thus is deprecated. | Covered | `removed.pgsql_libpq_version_str` | PGSQL_LIBPQ_VERSION_STR is reported. |
| PGSQL / PDO PGSQL | Function aliases in the pgsql extension have been deprecated. | Covered | `deprecated.pg_alias` | pg_numrows() is reported. |
| Zip | Using empty file as ZipArchive is deprecated. | Runtime only |  | Whether the opened file is empty depends on runtime data. |
| Zip | The procedural API of Zip is deprecated. | Covered | `deprecated.zip_procedural` | zip_open() and zip_read() are reported. |
| Reflection | ReflectionFunction::isDisabled() is deprecated, as it is no longer possible to create a ReflectionFunction for a disabled function. | Covered | `deprecated.reflection_method` | ReflectionFunction::isDisabled() is reported. |
| Reflection | ReflectionParameter::getClass(), ReflectionParameter::isArray(), and ReflectionParameter::isCallable() are deprecated. | Covered | `deprecated.reflection_method` | getClass(), isArray() and isCallable() on a ReflectionParameter are reported. |
| XML-RPC | The xmlrpc extension has been unbundled and moved to PECL. | Covered | `removed.xmlrpc` | xmlrpc_encode() is reported. |

## PHP 8.1

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-8.1/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | Access to the $GLOBALS array is now subject to a number of restrictions. | Covered | `removed.globals_write` | array_pop($GLOBALS) and $GLOBALS = [] are reported. |
| Core | Passing null to a non-nullable argument of a built-in function is deprecated. | Covered | `deprecated.null_to_internal` | str_contains('foobar', null) and strlen(null) are reported; null from variables is runtime. |
| Core | When a method using static variables is inherited (but not overridden), the inherited method will now share static variables with the parent method. | Gap |  | Not reported: class A { static function counter() { static $c = 0; } } class B extends A {} B::counter(); shares the static now. |
| Core | Most non-final internal methods now require overriding methods to declare a compatible return type, otherwise a deprecated notice is emitted during inheritance validation. | Covered | `deprecated.tentative_return_type` | Override of ArrayIterator::current() without a return type is reported. |
| Fileinfo | The fileinfo functions now accept and return, respectively, finfo objects instead of resources. | Covered | `removed.resource_check` | is_resource(finfo_open()) result is reported. |
| FTP | The FTP functions now accept and return, respectively, FTP\Connection objects instead of resources. | Covered | `removed.resource_check` | is_resource(ftp_connect()) result is reported. |
| IMAP | The IMAP functions now accept and return, respectively, IMAP\Connection objects instead of resources. | Covered | `removed.imap`<br>`removed.resource_check` | is_resource(imap_open()) result is reported, along with the imap removal. |
| LDAP | The LDAP functions now accept and return, respectively, LDAP\Connection objects instead of "ldap link" resources. | Covered | `removed.resource_check` | is_resource(ldap_connect()) result is reported. |
| LDAP | The LDAP functions now accept and return, respectively, LDAP\Result objects instead of "ldap result" resources. | Covered | `removed.resource_check` | is_resource() on ldap_search(), ldap_list() and ldap_read() results is reported. |
| LDAP | The LDAP functions now accept and return, respectively, LDAP\ResultEntry objects instead of "ldap result entry" resources. | Covered | `removed.resource_check` | is_resource() on ldap_first_entry() and ldap_next_entry() results is reported. |
| MySQLi | mysqli_fetch_fields() and mysqli_fetch_field_direct() will now always return zero for max_length. | Covered | `behavior.mysqli_max_length` | max_length read from a mysqli_fetch_field*() or ->fetch_field*() result is reported. |
| MySQLi | The MYSQLI_STMT_ATTR_UPDATE_MAX_LENGTH option no longer has an effect. | Covered | `deprecated.mysqli_update_max_length` | MYSQLI_STMT_ATTR_UPDATE_MAX_LENGTH is reported. |
| MySQLi | The MYSQLI_STORE_RESULT_COPY_DATA option no longer has an effect. | Covered | `deprecated.mysqli_store_result_copy_data` | MYSQLI_STORE_RESULT_COPY_DATA constant use is reported. |
| MySQLi | The default error handling mode has been changed from "silent" to "exceptions". | Gap |  | Not reported: mysqli_connect()/mysqli_query() results checked with === false and no mysqli_report() call, which now throw instead. |
| MySQLi | Classes extending mysqli_stmt::execute() will be required to specify the additional parameter now. | Covered | `removed.incompatible_override` | class S extends mysqli_stmt { function execute() {} } is reported. |
| MySQLi | mysqli::connect() will now return true instead of null on success. | Gap |  | Not reported: if ($m->connect('h','u','p') === null) on a new mysqli object; connect() now returns true. |
| MySQLnd | The mysqlnd.fetch_data_copy ini setting has been removed. | Covered | `removed.builtin_ini` | mysqlnd.fetch_data_copy is reported in php.ini, .user.ini, .htaccess and ini_set(). |
| OpenSSL | EC private keys will now be exported in PKCS#8 format rather than traditional format, just like all other keys. | Runtime only |  | Export format depends on the key type created at runtime. |
| OpenSSL | openssl_pkcs7_encrypt() and openssl_cms_encrypt() will now default to using AES-128-CBC rather than RC2-40. | Covered | `behavior.openssl_default_cipher` | openssl_pkcs7_encrypt('in', 'out', $cert, []) is reported. |
| PDO | PDO::ATTR_STRINGIFY_FETCHES now also stringifies values of type bool to "0" or "1". | Runtime only |  | Only matters when fetched column values are booleans, which depends on the database data. |
| PDO | Calling bindColumn() with PDO::PARAM_LOB (and assuming stringification is not enabled) will now consistently bind a stream result, as documented. | Covered | `behavior.bind_column_lob` | function f(PDOStatement $stmt) { $stmt->bindColumn(1, $lob, PDO::PARAM_LOB); } is reported when the receiver's class is visible in the code. |
| PDO MySQL | Integers and floats in result sets will now be returned using native PHP types instead of strings when using emulated prepared statements. | Runtime only |  | Native int/float results depend on the driver, emulation setting and column types at runtime. |
| PDO SQLite | Integers and floats in results sets will now be returned using native PHP types. | Runtime only |  | Native int/float results from PDO SQLite depend on the stored data at runtime. |
| PgSQL | The PgSQL functions now accept and return, respectively, \PgSql\Connection objects instead of "pgsql link" resources. | Covered | `removed.resource_check` | is_resource(pg_connect()) result is reported. |
| PgSQL | The PgSQL functions now accept and return, respectively, \PgSql\Result objects instead of "pgsql result" resources. | Covered | `removed.resource_check` | is_resource(pg_query()) result is reported. |
| PgSQL | The PgSQL functions now accept and return, respectively, \PgSql\Lob objects instead of "pgsql large object" resources. | Covered | `removed.resource_check` | is_resource() on pg_lo_open() results is reported. |
| Phar | To comply with the ArrayAccess interface, Phar::offsetUnset() and PharData::offsetUnset() no longer return a boolean. | Runtime only |  | Using the return value of ->offsetUnset() needs the receiver to be known as Phar/PharData; the method name is generic ArrayAccess. |
| PSpell | The PSpell functions now accept and return, respectively, PSpell\Dictionary objects instead of "pspell" resources. | Covered | `removed.pspell`<br>`removed.resource_check` | is_resource(pspell_new()) result is reported, plus the pspell removal. |
| PSpell | The PSpell functions now accept and return, respectively, PSpell\Config objects instead of "pspell config" resources. | Covered | `removed.pspell` | pspell_config_create() is reported via the pspell removal, though the is_resource() check on it is not tracked. |
| Standard | version_compare() no longer accepts undocumented operator abbreviations. | Covered | `removed.version_compare_operator` | version_compare('1.0', '2.0', 'l') is reported. |
| Standard | htmlspecialchars(), htmlentities(), htmlspecialchars_decode(), html_entity_decode() and get_html_translation_table() now use ENT_QUOTES \| ENT_SUBSTITUTE rather than ENT_COMPAT by default. | Gap |  | Statically detectable, but deliberately not reported: one-argument calls matched 445 lines across the corpus, and for most of them the only difference is that single quotes are now encoded. |
| Standard | debug_zval_dump() will now print reference wrappers with their refcount, instead of only prepending a "&" to the value. | Not applicable |  | debug_zval_dump() debugging output format only. |
| Standard | debug_zval_dump() will now print "interned" instead of a dummy refcount of one for interned strings and immutable arrays. | Not applicable |  | debug_zval_dump() debugging output format only. |
| SPL | SplFixedArray will now be JSON encoded like an array. | Runtime only |  | json_encode() of an SplFixedArray depends on the runtime value type. |
| Core | Implementing the Serializable interface without also implementing __serialize() and __unserialize() has been deprecated. | Covered | `deprecated.serializable` | Class implementing Serializable without __serialize/__unserialize is reported. |
| Core | Implicit conversion of floats to integers that result in loss of precision, e.g. a truncation from 1.9 to 1, is deprecated. | Covered | `deprecated.implicit_float_to_int` | Literal fractional array keys and % operands are reported; values that only exist at runtime are not. |
| Core | Calling a static method or accessing a static property directly on a trait is deprecated. | Covered | `deprecated.trait_static_access` | trait T { static function f() {} } T::f(); is reported. |
| Core | Returning a non-array from __sleep will raise a warning | Covered | `behavior.sleep_non_array` | __sleep() returning a scalar literal such as null or false is reported. |
| Core | Returning by reference from a void function is deprecated. | Covered | `deprecated.void_by_reference` | function &f(): void {} is reported. |
| Core | Automatic conversion of "false" into an empty array on write operands is deprecated. | Gap |  | Mostly runtime, but literal $a = false; $a[] = 1; is not reported. |
| Ctype | Passing a non-string value to ctype_*() functions is deprecated. | Covered | `deprecated.ctype_non_string` | ctype_digit(5) is reported. |
| Date | The date_sunrise() and date_sunset() functions have been deprecated in favor of date_sun_info(). | Covered | `deprecated.date_sunrise`<br>`deprecated.date_sunset` | date_sunrise() and date_sunset() calls are reported. |
| Date | The strftime() and gmstrftime() functions have been deprecated in favor of date()/DateTime::format() (for locale-independent formatting) or IntlDateFormatter::format() (for locale-dependent… | Covered | `deprecated.strftime`<br>`deprecated.gmstrftime` | strftime() and gmstrftime() calls are reported. |
| Filter | The FILTER_SANITIZE_STRING and FILTER_SANITIZE_STRIPPED filters have been deprecated. | Covered | `deprecated.filter_sanitize_string`<br>`deprecated.filter_sanitize_stripped` | Both filter constants are reported. |
| Filter | The filter.default ini setting is deprecated. | Covered | `deprecated.ini_setting` | filter.default in php.ini and in ini_set() is reported. |
| GD | The $num_points parameter of image(open\|filled)polygon has been deprecated. | Covered | `deprecated.polygon_num_points` | imagepolygon($im, $pts, 3, $col) is reported. |
| Hash | The mhash(), mhash_keygen_s2k(), mhash_count(), mhash_get_block_size() and mhash_get_hash_name() functions are deprecated. | Covered | `deprecated.mhash`<br>`deprecated.mhash_constant` | mhash(), mhash_count() and MHASH_* constants are reported. |
| IMAP | The NIL constant has been deprecated. | Partial | `removed.imap` | IMAP function calls are reported via the imap removal, but a standalone NIL constant (echo NIL;) is not. |
| Intl | Calling IntlCalendar::roll() with bool argument is deprecated. | Covered | `deprecated.intlcal_roll_bool` | $cal = new IntlGregorianCalendar(); $cal->roll(IntlCalendar::FIELD_DAY_OF_MONTH, true) is reported when the receiver's class is visible in the code. |
| Mbstring | Calling mb_check_encoding() without an argument is deprecated. | Covered | `deprecated.implicit_argument` | mb_check_encoding() without arguments is reported. |
| MySQLi | The mysqli_driver::$driver_version property has been deprecated. | Covered | `deprecated.mysqli_driver_version` | $d = new mysqli_driver(); echo $d->driver_version is reported when the receiver's class is visible in the code. |
| MySQLi | Calling mysqli::get_client_info in OO style or passing $mysqli argument to mysqli_get_client_info() function has been deprecated. | Covered | `deprecated.mysqli_method`<br>`deprecated.mysqli_get_client_info_arg` | mysqli_get_client_info($m) is reported. |
| MySQLi | The mysqli::init() method has been deprecated. | Covered | `deprecated.builtin_method` | parent::init() in a mysqli subclass is reported. |
| OCI8 | The INI directive oci8.old_oci_close_semantics has been deprecated. | Covered | `deprecated.ini_setting` | ini_set('oci8.old_oci_close_semantics', 1) is reported. |
| ODBC | odbc_result_all() has been deprecated. | Covered | `deprecated.odbc_result_all` | odbc_result_all() call is reported. |
| PDO | The PDO::FETCH_SERIALIZE mode has been deprecated. | Covered | `deprecated.pdo_fetch_serialize` | PDO::FETCH_SERIALIZE is reported. |
| PgSQL | Not passing the connection argument to PgSQL functions and using the default connection is deprecated. | Covered | `deprecated.pg_implicit_connection` | pg_query('SELECT 1') is reported. |
| SOAP | The ssl_method option for the SoapClient constructor has been deprecated in favor of ssl stream context options. | Covered | `deprecated.soap_ssl_method` | new SoapClient('x.wsdl', ['ssl_method' => SOAP_SSL_METHOD_TLS]) is reported. |
| Standard | Calling key(), current(), next(), prev(), reset(), or end() on objects is deprecated. | Covered | `deprecated.array_pointer_on_object` | key(), current(), next(), prev(), reset() and end() on $this or a new object are reported; other objects need the argument type. |
| Standard | The strptime() function has been deprecated. | Covered | `deprecated.strptime` | strptime() call is reported. |
| Standard | The auto_detect_line_endings ini setting has been deprecated. | Covered | `deprecated.ini_setting` | auto_detect_line_endings in php.ini and ini_set() is reported. |
| Standard | The FILE_BINARY and FILE_TEXT constants are deprecated. | Covered | `deprecated.file_text`<br>`deprecated.file_binary` | FILE_TEXT and FILE_BINARY constants are reported. |

## PHP 8.2

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-8.2/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Date | DateTime::createFromImmutable() now has a tentative return type of static, previously it was DateTime. | Covered | `deprecated.tentative_return_type` | DateTime subclass overriding createFromImmutable(): DateTime is reported. |
| Date | DateTimeImmutable::createFromMutable() now has a tentative return type of static, previously it was DateTimeImmutable. | Covered | `deprecated.tentative_return_type` | DateTimeImmutable subclass overriding createFromMutable(): DateTimeImmutable is reported. |
| ODBC | The ODBC extension now escapes the username and password for the case when both a connection string and username/password are passed, and the string must be appended to. | Runtime only |  | Escaping only matters when runtime username/password values contain special characters. |
| PDO_ODBC | The PDO_ODBC extension also escapes the username and password when a connection string is passed. | Runtime only |  | Escaping only matters when runtime username/password values contain special characters. |
| Standard | glob() returns empty array if all paths are restricted by open_basedir. | Runtime only |  | Depends on the open_basedir configuration and runtime paths. |
| Standard | strtolower() and strtoupper() are no longer locale-sensitive. | Runtime only |  | Only differs when a non-C locale is active at runtime and input has non-ASCII bytes. |
| Standard | str_split() returns an empty array for an empty string now. | Runtime only |  | Only differs when the string passed to str_split() is empty at runtime. |
| Standard | ksort() and krsort() do numeric string comparison under SORT_REGULAR using the standard PHP 8 rules now. | Runtime only |  | Depends on the runtime keys being numeric strings. |
| SPL | The following methods now enforce their signature: | Covered | `removed.splfileobject_extra_argument` | $f = new SplFileObject('x'); $f->ftell(1) is reported when the receiver's class is visible in the code. |
| SPL | SplFileObject::hasChildren() now has a tentative return type of false, previously it was bool | Covered | `deprecated.tentative_return_type` | SplFileObject subclass overriding hasChildren(): bool is reported. |
| SPL | SplFileObject::getChildren() now has a tentative return type of null, previously it was ?RecursiveIterator | Covered | `deprecated.tentative_return_type` | SplFileObject subclass overriding getChildren(): ?RecursiveIterator is reported. |
| SPL | GlogIterator returns empty array if all paths are restricted by open_basedir. | Runtime only |  | Depends on the open_basedir configuration and runtime paths. |
| Core | Creation of dynamic properties is deprecated, unless the class opts in by using the #[AllowDynamicProperties] attribute. | Covered | `deprecated.dynamic_property` | $this->x = 1 in a class without the declaration is reported. |
| Core | Callables that are not accepted by the $callable() syntax (but are accepted by call_user_func) are deprecated. | Covered | `deprecated.partial_callable` | 'self::g', ['parent', 'g'] and 'static::g' callables are reported. |
| Core | The "${var}" and "${expr}" style string interpolations are deprecated and will be removed in PHP 9. | Covered | `deprecated.dollar_brace_interpolation` | "${v}" and "${'v'}" are reported. |
| Mbstring | Use of QPrint, Base64, Uuencode, and HTML-ENTITIES 'text encodings' is deprecated for all Mbstring functions. | Covered | `deprecated.mb_encoding` | HTML-ENTITIES, BASE64 and Quoted-Printable literal encodings are reported. |
| SPL | The SplFileInfo::_bad_state_ex() internal method has been deprecated. | Covered | `deprecated.splfileinfo_bad_state_ex` | ->_bad_state_ex() on an SplFileInfo object is reported. |
| Standard | utf8_encode() and utf8_decode() have been deprecated. | Covered | `deprecated.utf8_encode`<br>`deprecated.utf8_decode` | utf8_encode() and utf8_decode() calls are reported. |

## PHP 8.3

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-8.3/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | Programs that were very close to overflowing the call stack may now throw an Error when using more than `zend.max_allowed_stack_size-zend.reserved_stack_size` bytes of stack… | Runtime only |  | Depends on runtime stack depth. |
| Core | Executing proc_get_status() multiple times will now always return the right value on posix systems. | Runtime only |  | Depends on runtime process status call sequence. |
| Core | Zend Max Execution Timers is now enabled by default for ZTS builds on Linux. | Not applicable |  | ZTS build / SAPI internals. |
| Core | Uses of traits with static properties will now redeclare static properties inherited from the parent class. | Covered | `behavior.trait_static_property` | A class using a trait whose static property an ancestor class also declares is reported. |
| Core | Assigning a negative index n to an empty array will now make sure that the next index is n+1 instead of 0. | Gap |  | Not reported: $a = []; $a[-5] = 'a'; $a[] = 'b'; or [-5 => 'a', 'b'] where the next key is now -4. |
| Core | Class constant visibility variance is now correctly checked when inherited from interfaces. | Partial | `removed.interface_constant_visibility` | Reported when the interface is declared in the same file; an interface from another file is not checked. |
| Core | WeakMaps entries whose key maps to itself (possibly transitively) may now be removed during cycle collection if the key is not reachable except by iterating over the WeakMap (reachability via… | Runtime only |  | Garbage-collection behavior of WeakMap entries at runtime. |
| Core | In addition to whitespace characters, now comments are allowed between `yield` and `from`. | Not applicable |  | yield /* comment */ from was a syntax error before; only tokenizer output changes. |
| DOM | DOMChildNode::after(), DOMChildNode::before(), DOMChildNode::replaceWith() on a node that has no parent is now a no-op instead of a hierarchy exception, which is the behaviour spec demands. | Runtime only |  | Depends on whether the DOM node has a parent at runtime. |
| DOM | Using the DOMParentNode and DOMChildNode methods without a document now works instead of throwing a HIERARCHY_REQUEST_ERR DOMException. | Runtime only |  | Depends on DOM node document state at runtime. |
| DOM | createAttributeNS() without specifying a prefix would incorrectly create a default namespace, placing the element inside the namespace instead of the attribute. | Runtime only |  | DOM namespace bugfix depends on runtime arguments and document. |
| DOM | createAttributeNS() would previously incorrectly throw a NAMESPACE_ERR when the prefix was already used for a different uri. | Runtime only |  | DOM namespace bugfix depends on runtime prefix conflicts. |
| DOM | New methods and properties were added to some DOM classes. | Covered | `removed.incompatible_override` | DOMElement subclass declaring incompatible toggleAttribute() / contains() is reported. |
| FFI | C functions that have a return type of void now return null instead of returning the following object object(FFI\CData:void) { } | Runtime only |  | FFI return value depends on runtime C function signatures. |
| Opcache | The opcache.consistency_checks INI directive was removed. | Covered | `removed.builtin_ini` | opcache.consistency_checks is reported in php.ini, .user.ini, .htaccess and ini_set(). |
| Phar | The type of Phar class constants are now declared. | Covered | `removed.typed_constant_override` | class P extends Phar { const SHA1 = []; } is reported. |
| Standard | The range() function has had various changes: | Covered | `removed.range_invalid` | Literal range() calls with a step of 0, an empty string or an array boundary are reported. |
| Standard | The file() flags error check now catches all invalid flags. | Covered | `removed.file_append_flag` | file('x', FILE_APPEND) is reported. |
| SNMP | The type of SNMP class constants are now declared. | Gap |  | Not reported: class S extends SNMP { const VERSION_1 = 'x'; } redeclaring a now-typed constant with another type. |
| Deprecated Functionality | Using the ++ operator on empty, non-numeric, or non-alphanumeric strings is now deprecated. | Runtime only |  | Whether the incremented string is empty or non-alphanumeric depends on runtime values. |
| Deprecated Functionality | Using the -- operator on empty or non-numeric strings is now deprecated. | Runtime only |  | Whether the decremented string is empty or non-numeric depends on runtime values. |
| Deprecated Functionality | Calling get_class() and get_parent_class() without arguments is now deprecated. | Covered | `deprecated.get_class_no_args` | get_class() and get_parent_class() without arguments are reported. |
| Deprecated Functionality | Calling dba_fetch() with $dba as the 3rd argument is now deprecated. | Covered | `deprecated.dba_fetch_order` | dba_fetch('k', 0, $dba) is reported. |
| Deprecated Functionality | Calling FFI::cast(), FFI::new(), and FFI::type() statically is now deprecated. | Covered | `deprecated.ffi_static_call` | Static FFI::cast(), FFI::new() and FFI::type() calls are reported. |
| Deprecated Functionality | The U_MULTIPLE_DECIMAL_SEP*E*RATORS constant had been deprecated, using the U_MULTIPLE_DECIMAL_SEP*A*RATORS instead is recommended. | Covered | `deprecated.u_multiple_decimal_seperators` | U_MULTIPLE_DECIMAL_SEPERATORS constant is reported. |
| Deprecated Functionality | The NumberFormatter::TYPE_CURRENCY has been deprecated. | Covered | `deprecated.numberformatter_type_currency` | NumberFormatter::TYPE_CURRENCY is reported. |
| Deprecated Functionality | Calling ldap_connect() with separate hostname and port is deprecated. | Covered | `deprecated.ldap_connect_port` | ldap_connect('h', 389) is reported. |
| Deprecated Functionality | Passing a negative $width to mb_strimwidth() is now deprecated. | Covered | `deprecated.mb_strimwidth_negative` | mb_strimwidth($s, 0, -5) is reported. |
| Deprecated Functionality | Calling Phar::setStub() with a resource and a length is now deprecated. | Covered | `deprecated.phar_setstub_length` | $p = new Phar('x.phar'); $p->setStub($fp, 100) is reported when the receiver's class is visible in the code. |
| Deprecated Functionality | The MT_RAND_PHP Mt19937 variant is deprecated. | Covered | `deprecated.mt_rand_php` | mt_srand(1, MT_RAND_PHP) is reported. |
| Standard | The assert_options() function is now deprecated. | Covered | `deprecated.assert_options` | assert_options() call is reported. |
| Standard | The ASSERT_ACTIVE, ASSERT_BAIL, ASSERT_CALLBACK, ASSERT_EXCEPTION, and ASSERT_WARNING constants have been deprecated. | Covered | `deprecated.assert_constants` | ASSERT_ACTIVE/BAIL/CALLBACK/EXCEPTION/WARNING constants are reported. |
| Standard | Using exceptions is now preferred, warnings will be removed in the future. | Covered | `deprecated.sqlite3_exceptions_off` | $db = new SQLite3('x'); $db->enableExceptions(false) is reported when the receiver's class is visible in the code. |
| Zip | The ZipArchive::FL_RECOMPRESS constant is deprecated and will be removed in a future version of libzip | Covered | `deprecated.zip_fl_recompress` | ZipArchive::FL_RECOMPRESS is reported. |
| Changes to INI File Handling | The assert.* INI settings have been deprecated. | Covered | `deprecated.ini_setting` | assert.* settings in php.ini and ini_set('assert.exception', ...) are reported. |

## PHP 8.4

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-8.4/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | The type of PHP_DEBUG and PHP_ZTS constants changed to bool. | Covered | `behavior.php_debug_bool` | Strict comparisons of PHP_DEBUG or PHP_ZTS with a number are reported. |
| Core | The name of uploaded files and files created by the tempnam() function are now 13 bytes longer. | Runtime only |  | Longer tempnam()/upload file names only matter if code depends on the generated name length. |
| Core | Encountering recursion during comparison now results in a Error exception, rather than a fatal error. | Runtime only |  | Recursion during comparison depends on runtime object graphs. |
| Core | Indirect modification of readonly properties within __clone() is no longer allowed, e.g. $ref = &$this->readonly. | Covered | `removed.readonly_clone_reference` | Taking a reference to a readonly property of the class (including promoted ones) inside __clone() is reported. |
| Core | The exit (and die) language constructs now behave more like a function. | Runtime only |  | TypeErrors from exit()/die() depend on argument types; literal exit(true)/die(1.5)/exit(null) were not reported, but common int/string calls are unaffected. |
| Core | The E_STRICT constant was deprecated and its corresponding error level was removed. | Covered | `deprecated.e_strict` | E_STRICT usage in error_reporting() is reported. |
| Extension Class constants are now typed | Date | Covered | `removed.typed_constant_override` | class D extends DateTime { const ATOM = 5; } is reported. |
| Extension Class constants are now typed | Intl | Covered | `removed.typed_constant_override` | class C extends Collator { const DEFAULT_VALUE = 'x'; } is reported. |
| Extension Class constants are now typed | PDO | Covered | `removed.typed_constant_override` | class Q extends PDO { const PARAM_INT = 'x'; } is reported. |
| Extension Class constants are now typed | Reflection | Covered | `removed.typed_constant_override` | class R extends ReflectionMethod { const IS_PUBLIC = 'x'; } is reported. |
| Extension Class constants are now typed | SPL | Covered | `removed.typed_constant_override` | class A extends ArrayObject { const STD_PROP_LIST = 'x'; } is reported. |
| Extension Class constants are now typed | Sqlite | Covered | `removed.typed_constant_override` | class S extends SQLite3 { const OK = 'x'; } is reported. |
| Extension Class constants are now typed | XMLReader | Covered | `removed.typed_constant_override` | class X extends XMLReader { const ELEMENT = 'x'; } is reported. |
| Resource to Object conversions | DBA: . dba_open() and dba_popen() will now return Dba\Connection | Covered | `removed.resource_check` | is_resource() on a dba_open() result is reported. |
| Resource to Object conversions | ODBC: . odbc_connect() and odbc_pconnect() will now return Odbc\Connection . | Covered | `removed.resource_check` | is_resource() on odbc_connect()/odbc_exec() results is reported. |
| Resource to Object conversions | SOAP: . SoapClient::$httpurl is now a Soap\Url object rather than a resource. | Covered | `removed.resource_check` | $c = new SoapClient('x'); if (is_resource($c->httpurl)) {} is reported when the receiver's class is visible in the code. |
| New warnings and exceptions | Curl: . curl_multi_select throws a ValueError if the timeout argument if it's negative or greater than PHP_INT_MAX. | Covered | `removed.curl_multi_select_negative` | curl_multi_select($mh, -1) is reported. |
| New warnings and exceptions | GD: . imagejpeg/imagewebp/imagepng/imageavif throws an exception if an invalid quality parameter value is passed. | Covered | `removed.image_quality_range` | imagejpeg($im, null, 150) is reported. |
| New warnings and exceptions | Gettext: . bind_textdomain_codeset, textdomain and d(*)gettext functions now throw an exception if the domain argument is empty. | Covered | `removed.gettext_empty_domain` | textdomain('') is reported. |
| New warnings and exceptions | Intl: . IntlCalendar::set(), IntlCalendar::setDate(), IntlCalendar::setDateTime(), and IntlGregorianCalendar date/time construction now report the correct argument position for out-of-bounds… | Runtime only |  | Out-of-bounds integer values and ResourceBundle offsets depend on runtime values. |
| New warnings and exceptions | MBString: . mb_encode_numericentity() and mb_decode_numericentity() now check that the $map is only composed of integers, if not a ValueError is thrown. | Runtime only |  | Map entry types, mb_http_input type and null bytes in encodings normally come from runtime data. |
| New warnings and exceptions | ODBC: . odbc_fetch_row() now emits a warning when a value less than or equal to 0 is passed for parameter $row. | Covered | `removed.odbc_row_zero` | odbc_fetch_row($r, 0) is reported. |
| New warnings and exceptions | PCNTL: . The functions pcntl_sigprocmask(), pcntl_sigwaitinfo() and pcntl_sigtimedwait() now throw: | Runtime only |  | Empty or invalid signal arrays depend on runtime values. |
| New warnings and exceptions | SimpleXML: . Calling simplexml_import_dom() with a non-XML object now throws a TypeError instead of a ValueError. | Runtime only |  | Only the thrown exception class changes for invalid runtime objects. |
| New warnings and exceptions | Standard: . round() now validates the value of the $mode parameter and throws a ValueError for invalid modes. | Covered | `removed.round_invalid_mode`<br>`removed.php_uname_mode`<br>`removed.csv_separator_length`<br>`behavior.unserialize_allowed_classes_type` | Literal invalid round() and php_uname() modes, CSV separators that aren't one character and a string allowed_classes are reported. |
| New warnings and exceptions | XMLReader: . Passing an invalid character encoding to XMLReader::open() or XMLReader::XML() now throws a ValueError. | Runtime only |  | Invalid encodings or null bytes passed to XMLReader come from runtime values. |
| New warnings and exceptions | XMLWriter: . Passing an encoding containing null bytes previously emitted a warning and now throws a ValueError. | Runtime only |  | Null bytes in XMLWriter encoding depend on runtime values. |
| New warnings and exceptions | XSL: . XSLTProcessor::setParameter() will now throw a ValueError when its arguments contain null bytes. | Runtime only |  | Null bytes in XSLT parameters and non-XML objects depend on runtime values. |
| DOM | Some DOM methods previously returned false or a PHP_ERR DOMException if a new node could not be allocated. | Not applicable |  | Node allocation failure is practically unreachable; only a tentative return type changed. |
| DOM | Previously, DOMXPath objects could be cloned, but resulted in an unusable object. | Covered | `removed.uncloneable_object` | $x = new DOMXPath($doc); $y = clone $x is reported when the receiver's class is visible in the code. |
| DOM | Removed DOMImplementation::getFeature(). | Covered | `removed.builtin_method` | DOMImplementation::getFeature() call is reported. |
| GMP | The GMP class is now final and cannot be extended anymore. | Covered | `removed.extends_final_class` | Extending GMP, and any other built-in class that became final, is reported. |
| MBString | On invalid strings (those with encoding errors), mb_substr() now interprets character indices in the same manner as most other mbstring functions. | Runtime only |  | mb_substr() index semantics on invalid strings depend on runtime string contents. |
| MBString | For SJIS-Mac (MacJapanese) strings, character indices passed to mb_substr() now refer to the indices of the Unicode codepoints which are produced when the string is converted to Unicode. | Runtime only |  | SJIS-Mac index semantics depend on runtime string contents. |
| Mysqli | The unused and undocumented constant MYSQLI_SET_CHARSET_DIR has been removed. | Covered | `removed.builtin_constant` | MYSQLI_SET_CHARSET_DIR is reported. |
| Mysqli | The MYSQLI_STMT_ATTR_PREFETCH_ROWS constant has been removed. | Covered | `removed.builtin_constant` | MYSQLI_STMT_ATTR_PREFETCH_ROWS is reported. |
| Mysqli | The MYSQLI_CURSOR_TYPE_FOR_UPDATE and MYSQLI_CURSOR_TYPE_SCROLLABLE constants have been removed. | Covered | `removed.builtin_constant` | MYSQLI_CURSOR_TYPE_FOR_UPDATE and MYSQLI_CURSOR_TYPE_SCROLLABLE are reported. |
| Mysqli | The unused MYSQLI_TYPE_INTERVAL constant, which is currently a stub and an alias for MYSQLI_TYPE_ENUM, has been removed. | Covered | `removed.builtin_constant` | MYSQLI_TYPE_INTERVAL is reported. |
| MySQLnd | The error code reported for MySQL server wait timeouts has been changed from 2006 to 4031 for MySQL server versions 8.0.24 and above. | Runtime only |  | The error code comes from the server connection at runtime. |
| Opcache | The JIT config defaults changed from opcache.jit=tracing and opcache.jit_buffer_size=0 to opcache.jit=disable and opcache.jit_buffer_size=64M, respectively. | Covered | `behavior.opcache_jit_default` | An ini file that sets opcache.jit_buffer_size without opcache.jit is reported. |
| Opcache | The maximum value of the opcache.interned_strings_buffer setting on 64bit architectures is now 32767 (it was previously 4095). | Not applicable |  | Raised maximum for opcache.interned_strings_buffer does not break existing configs. |
| Opcache | If JIT is enabled, PHP will now exit with a fatal error on startup in case of JIT startup initialization issues. | Not applicable |  | JIT startup failure behavior is environment-specific, not source-level. |
| PCNTL | The functions pcntl_sigprocmask(), pcntl_sigwaitinfo() and pcntl_sigtimedwait() now always return false on failure. | Covered | `behavior.pcntl_failure_value` | pcntl_sigwaitinfo() and pcntl_sigtimedwait() results compared with -1 are reported. |
| PCRE | The bundled pcre2lib has been updated to version 10.44. | Covered | `behavior.regex_empty_min_quantifier` | Literal preg_*() patterns containing {,n} are reported. |
| PDO_DBLIB | setAttribute, DBLIB_ATTR_STRINGIFY_UNIQUEIDENTIFIER and DBLIB_ATTR_DATETIME_CONVERT have been changed to set value as a bool. | Runtime only |  | setAttribute value types for DBLIB attributes depend on runtime values. |
| PDO_FIREBIRD | Since some Firebird C++ APIs are used now, this extension requires a C++ compiler to be built. | Not applicable |  | Build requirement for PDO_FIREBIRD. |
| PDO_FIREBIRD | getAttribute, ATTR_AUTOCOMMIT has been changed to get the value as a bool. | Runtime only |  | getAttribute() now returns bool; impact depends on how the value is compared at runtime. |
| PDO_MYSQL | getAttribute, ATTR_AUTOCOMMIT, ATTR_EMULATE_PREPARES, MYSQL_ATTR_DIRECT_QUERY have been changed to get values as bool. | Runtime only |  | getAttribute() now returns bool; impact depends on how the value is compared at runtime. |
| PDO_PGSQL | The DSN's credentials, when set, are given priority over their PDO constructor counterparts, being closer to the documentation states. | Runtime only |  | DSN credential priority depends on runtime DSN/argument values. |
| SimpleXML | Get methods called, or casting to a string on a SimpleXMLElement will no longer implicitly reset the iterator data, unless explicitly rewound. | Runtime only |  | SimpleXML iterator reset behavior depends on runtime iteration. |
| SOAP | SoapClient::$typemap is now an array rather than a resource. | Covered | `removed.resource_check` | $c = new SoapClient('x'); if (is_resource($c->typemap)) {} is reported when the receiver's class is visible in the code. |
| SOAP | The SOAP extension gained an optional dependency on the session extension. | Not applicable |  | Build/rtld-now configuration issue. |
| Standard | strcspn() with empty $characters now returns the length of the string instead of incorrectly stopping at the first NUL character. | Runtime only |  | strcspn() with empty characters normally comes from a variable. |
| Standard | http_build_query() now correctly handles backed enums. | Runtime only |  | Backed enums in http_build_query() data depend on runtime values. |
| Standard | stream_bucket_make_writeable() and stream_bucket_new() will now return a StreamBucket instance instead of an instance of stdClass. | Runtime only |  | StreamBucket vs stdClass only matters for runtime type checks inside stream filters. |
| Tidy | Failures in the constructor now throw exceptions rather than emitting warnings and having a broken object. | Runtime only |  | Tidy constructor failures depend on runtime input. |
| XML | The xml_set_*_handler() functions now declare and check for an effective signature of callable\|string\|null for the $handler parameters. | Partial | `deprecated.xml_set_object` | xml_set_object() is reported, but calling xml_set_*_handler() with method-name strings before xml_set_object() and passing '' are not. |
| Core | Implicitly nullable parameter types are now deprecated. | Covered | `deprecated.implicit_nullable` | `int $x = null` parameter is reported. |
| Core | Passing E_USER_ERROR to trigger_error() is now deprecated. | Covered | `deprecated.e_user_error` | trigger_error('x', E_USER_ERROR) is reported. |
| Core | Using "_" as a class name is now deprecated. | Covered | `deprecated.underscore_class_name` | class _ {} is reported. |
| Core | Raising zero to the power of negative number is deprecated. | Covered | `deprecated.pow_zero_negative` | 0 ** -1 and pow(0, -2) with literals are reported. |
| Core | The E_STRICT constant was deprecated and its corresponding error level was removed. | Covered | `deprecated.e_strict` | E_STRICT constant use is reported. |
| Curl | The CURLOPT_BINARYTRANSFER constant is deprecated. | Covered | `deprecated.curlopt_binarytransfer` | CURLOPT_BINARYTRANSFER is reported. |
| Date | Calling DatePeriod::__construct(string $isostr, int $options = 0) is deprecated. | Covered | `deprecated.date_period_iso_string` | new DatePeriod('R4/2012-07-01T00:00:00Z/P7D') is reported. |
| Date | Constants SUNFUNCS_RET_TIMESTAMP, SUNFUNCS_RET_STRING, and SUNFUNCS_RET_DOUBLE are now deprecated, following the deprecation of the associated date_sunset() and date_sunrise() functions in PHP 8.1. | Covered | `deprecated.sunfuncs_ret` | All three SUNFUNCS_RET_* constants are reported. |
| DBA | Passing null or false to dba_key_split() is deprecated. | Covered | `deprecated.dba_key_split_null` | dba_key_split(null) is reported. |
| DOM | Deprecated DOM_PHP_ERR constant. | Covered | `deprecated.dom_php_err` | DOM_PHP_ERR is reported. |
| DOM | DOMDocument::$actualEncoding, DOMDocument::config, DOMEntity::$actualEncoding, DOMEntity::$encoding, DOMEntity::$version have been deprecated. | Covered | `deprecated.dom_document_property` | $doc = new DOMDocument(); echo $doc->actualEncoding is reported when the receiver's class is visible in the code. |
| Hash | Deprecated passing incorrect data types for options to ext/hash functions. | Covered | `deprecated.hash_option_type` | A string seed option passed to hash functions is reported. |
| Intl | Calling intlcal_set() as well as calling IntlCalendar::set() with more than 2 arguments is deprecated. | Covered | `deprecated.intlcal_set` | $cal = new IntlGregorianCalendar(); $cal->set(2020, 1, 1) is reported when the receiver's class is visible in the code. |
| Intl | Calling intlgregcal_create_instance() as well as calling IntlGregorianCalendar::__construct() with more than 2 arguments is deprecated. | Covered | `deprecated.intlgregcal_create_instance` | new IntlGregorianCalendar(2020, 1, 1) is reported. |
| LDAP | Calling ldap_connect() with more than 2 arguments is deprecated. | Covered | `removed.builtin_arity` | ldap_connect() with 3 arguments is reported (as an arity issue). |
| LDAP | Calling ldap_exop() with more than 4 arguments is deprecated. | Covered | `deprecated.ldap_exop_response` | ldap_exop($l, $o, null, null, $r, $m) is reported. |
| Mysqli | The mysqli_ping() function and mysqli::ping() method are now deprecated, as the reconnect feature was removed in PHP 8.2. | Covered | `deprecated.mysqli_ping`<br>`deprecated.mysqli_method` | mysqli_ping() and $mysqli->ping() are reported. |
| Mysqli | The mysqli_kill() function and mysqli::kill() method are now deprecated. | Covered | `deprecated.mysqli_kill`<br>`deprecated.mysqli_method` | mysqli_kill() and $mysqli->kill() are reported. |
| Mysqli | The mysqli_refresh() function and mysqli::refresh() method are now deprecated. | Covered | `deprecated.mysqli_refresh`<br>`deprecated.mysqli_method`<br>`deprecated.mysqli_refresh_constant` | mysqli_refresh(), ->refresh() and MYSQLI_REFRESH_* constants are reported. |
| Mysqli | Passing explicitly the $mode parameter to mysqli_store_result() has been deprecated. | Covered | `deprecated.mysqli_store_result_copy_data`<br>`deprecated.mysqli_store_result_mode` | $m = new mysqli(); $m->store_result(0) is reported when the receiver's class is visible in the code. |
| PDO_PGSQL | Using escaped question marks (??) inside dollar-quoted strings is deprecated. | Covered | `deprecated.pgsql_escaped_question` | ?? inside $$-quoted SQL passed to prepare(), query() or exec() is reported. |
| PgSQL | Calling pg_fetch_result() with 2 arguments is deprecated. | Covered | `deprecated.pg_implicit_row` | pg_fetch_result($r, 'col') is reported. |
| PgSQL | Calling pg_field_prtlen() with 2 arguments is deprecated. | Covered | `deprecated.pg_implicit_row` | pg_field_prtlen($r, 'col') is reported. |
| PgSQL | Calling pg_field_is_null() with 2 arguments is deprecated. | Covered | `deprecated.pg_implicit_row` | pg_field_is_null($r, 'col') is reported. |
| Random | lcg_value() is deprecated, as the function is broken in multiple ways. | Covered | `deprecated.lcg_value` | lcg_value() is reported. |
| Reflection | Calling ReflectionMethod::__construct() with 1 argument is deprecated. | Covered | `deprecated.reflection_method_string` | new ReflectionMethod('A::b') is reported. |
| Session | Calling session_set_save_handler() with more than 2 arguments is deprecated. | Covered | `deprecated.session_set_save_handler` | session_set_save_handler() with six callables is reported. |
| Session | Changing the INI settings session.sid_length and session.sid_bits_per_character is deprecated. | Covered | `deprecated.ini_setting` | session.sid_length/sid_bits_per_character in ini_set() and php.ini are reported. |
| Session | Changing the INI settings session.use_only_cookies, session.use_trans_sid, session.trans_sid_tags, session.trans_sid_hosts, and session.referer_check is deprecated. | Partial | `deprecated.ini_setting` | session.use_trans_sid, referer_check and use_only_cookies are reported; the SID constant is not. |
| SOAP | Passing an int to SoapServer::addFunction() is now deprecated. | Covered | `deprecated.soap_functions_all` | $s = new SoapServer(null); $s->addFunction(999) is reported when the receiver's class is visible in the code. |
| SOAP | The SOAP_FUNCTIONS_ALL constant is now deprecated. | Covered | `deprecated.soap_functions_all` | SOAP_FUNCTIONS_ALL is reported. |
| SPL | The SplFixedArray::__wakeup() method has been deprecated as it implements __serialize() and __unserialize() which need to be overwritten instead. | Covered | `deprecated.builtin_method`<br>`deprecated.splfixedarray_wakeup` | Overriding and calling SplFixedArray::__wakeup() are reported. |
| SPL | Using the default value for $escape parameter of: | Covered | `deprecated.csv_escape` | $f = new SplFileObject('x'); $f->fgetcsv() is reported when the receiver's class is visible in the code. |
| Standard | Calling stream_context_set_option() with 2 arguments is deprecated. | Covered | `deprecated.stream_context_option_array` | stream_context_set_option($ctx, ['ssl' => []]) is reported. |
| Standard | Unserializing strings using the uppercase 'S' tag is deprecated. | Covered | `deprecated.unserialize_upper_s` | Literal serialized strings with the S: tag passed to unserialize() are reported. |
| Standard | Using the default value for $escape parameter of: | Covered | `deprecated.csv_escape` | fputcsv()/fgetcsv()/str_getcsv() without $escape are reported. |
| XML | The xml_set_object() function has been deprecated. | Covered | `deprecated.xml_set_object` | xml_set_object() is reported. |
| XML | Passing non-callable strings to the xml_set_*_handler() functions is now deprecated. | Partial | `deprecated.xml_set_object` | xml_set_object() is reported, but the xml_set_*_handler() calls with non-callable method-name strings or '' are not. |
| IMAP | The IMAP extension has been unbundled and moved to PECL. | Covered | `removed.imap` | imap_open() is reported. |
| OCI8 | The OCI8 extension has been unbundled and moved to PECL. | Covered | `removed.oci8` | oci_connect() is reported. |
| PDO_OCI | The PDO_OCI extension has been unbundled and moved to PECL. | Covered | `removed.pdo_oci` | new PDO('oci:dbname=x') is reported. |
| PSpell | The pspell extension has been unbundled and moved to PECL. | Covered | `removed.pspell` | pspell_new() is reported. |

## PHP 8.5

Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-8.5/UPGRADING)

| Area | Change | Status | Rules | Note |
| --- | --- | --- | --- | --- |
| Core | It is no longer possible to use "array" and "callable" as class alias names in class_alias(). | Covered | `removed.reserved_name` | class_alias() with array or callable as the alias is reported. |
| Core | Loosely comparing uncomparable objects (e.g. enums, \CurlHandle and other internal classes) to booleans was previously inconsistent. | Runtime only |  | Loose comparison of uncomparable objects to booleans depends on runtime types. |
| Core | The return value of gc_collect_cycles() no longer includes strings and resources that were indirectly collected through cycles. | Runtime only |  | gc_collect_cycles() return value depends on runtime state. |
| Core | It is now allowed to substitute static with self or the concrete class name in final subclasses. | Not applicable |  | Relaxation: substituting static with self in final classes was previously not allowed. |
| Core | The tick handlers are now deactivated after all shutdown functions, destructors have run and the output handlers have been cleaned up. | Runtime only |  | Tick handler timing during shutdown is runtime behavior. |
| Core | Traits are now bound before the parent class. | Runtime only |  | Trait vs parent binding order effects depend on subtle runtime linking. |
| Core | Errors emitted during compilation and class linking are now always delayed and handled after compilation or class linking. | Runtime only |  | Delayed compile/link error handling depends on runtime error handlers. |
| Core | Exceptions thrown by user-defined error handlers when handling class linking errors are not promoted to fatal errors anymore and do not prevent linking. | Runtime only |  | Error handler exceptions during linking depend on runtime handlers. |
| Core | Applying #[\Attribute] to an abstract class, enum, interface, or trait triggers an error during compilation. | Covered | `removed.attribute_on_non_class` | #[Attribute] interface I {} is reported. |
| Core | The disable_classes INI setting has been removed as it causes various engine assumptions to be broken. | Covered | `removed.builtin_ini` | disable_classes is reported in php.ini, .user.ini, .htaccess and ini_set(). |
| Core | Destructuring non-array values (other than NULL) using [] or list() now emits a warning. | Runtime only |  | Destructuring non-arrays usually involves variables; literal `[$a, $b] = 42` was not reported either. |
| Core | A warning is now emitted when casting floats to int if they cannot be represented as one. | Runtime only |  | Out-of-range float to int casts depend on runtime values. |
| Core | A warning is now emitted when casting NAN to other types. | Runtime only |  | NAN casts depend on runtime values; literal `(int) NAN` was not reported. |
| BZ2 | bzcompress() now throws a ValueError when $block_size is not between 1 and 9. | Covered | `removed.bzcompress_range` | bzcompress($s, 10) is reported. |
| BZ2 | bzcompress() now throws a ValueError when $work_factor is not between 0 and 250. | Covered | `removed.bzcompress_range` | bzcompress($s, 4, 300) is reported. |
| DOM | Cloning a DOMNamedNodeMap, DOMNodeList, Dom\NamedNodeMap, Dom\NodeList, Dom\HTMLCollection, and Dom\DtdNamedNodeMap now fails. | Covered | `removed.uncloneable_object` | clone of ->childNodes, ->attributes or ->getElementsByTagName() is reported. |
| FileInfo | finfo_file() and finfo::file() now throws a ValueError instead of a TypeError when $filename contains nul bytes. | Runtime only |  | Only the exception class changes for nul bytes in runtime filenames. |
| Intl | The extension now requires at least ICU 57.1. | Not applicable |  | ICU build requirement. |
| Intl | The behaviour of Collator::SORT_REGULAR with respect to handling numeric strings is now aligned with the behaviour of SORT_REGULAR in ext/standard. | Runtime only |  | Collator SORT_REGULAR results depend on runtime string values. |
| LDAP | ldap_get_option() and ldap_set_option() now throw a ValueError when passing an invalid option. | Runtime only |  | Invalid LDAP option values normally come from runtime values. |
| MBstring | Unicode data tables have been updated to Unicode 17.0 | Runtime only |  | Unicode table update affects results for runtime string data. |
| MySQLi | Calling the mysqli constructor on an already-constructed object is now no longer possible and throws an Error. | Runtime only |  | Re-constructing an existing mysqli object depends on runtime object state. |
| ODBC | ODBC now assumes that at least ODBC 3.5 functionality is available. | Not applicable |  | ODBC build definition change. |
| ODBC | ODBC no longer has build flags to build against specific drivers (except for DB2) and removes special cases for those drivers. | Not applicable |  | ODBC build flags change. |
| Opcache | The Opcache extension is now always built into the PHP binary and is always loaded. | Covered | `behavior.opcache_zend_extension` | zend_extension lines that load OPcache are reported. |
| PCNTL | pcntl_exec() now throws ValueErrors when entries of the $args parameter contain null bytes. | Runtime only |  | Null bytes in pcntl_exec() args depend on runtime values. |
| PCNTL | pcntl_exec() now throws ValueErrors when entries or keys of the $env_vars parameter contain null bytes. | Runtime only |  | Null bytes in pcntl_exec() env vars depend on runtime values. |
| PCRE | The extension is compiled without semi-deprecated PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK compile option. | Covered | `removed.regex_keep_in_lookaround` | Literal preg_*() patterns with \K inside a lookaround are reported. |
| PDO | The constructor arguments set in conjunction with PDO::FETCH_CLASS now follow the usual CUFA (call_user_func_array) semantics. | Covered | `removed.fetch_class_named_args` | function f(PDOStatement $s) { $s->setFetchMode(PDO::FETCH_CLASS, 'X', ['name' => 1]); } is reported when the receiver's class is visible in the code. |
| PDO | Attempting to call PDOStatement::setFetchMode during a call to PDO::fetch(), PDO::fetchObject(), PDO::fetchAll(), for example using tricks such as passing the statement object as a constructor… | Runtime only |  | Calling setFetchMode() during a fetch depends on runtime call flow. |
| PDO | The value of the constants PDO::FETCH_GROUP, PDO::FETCH_UNIQUE, PDO::FETCH_CLASSTYPE, PDO::FETCH_PROPS_LATE, and PDO::FETCH_SERIALIZE have changed. | Runtime only |  | Changed constant values only matter if code hardcodes or persists the raw integers. |
| PDO | A ValueError is now thrown if PDO::FETCH_PROPS_LATE is used with a fetch mode different than PDO::FETCH_CLASS, consistent with other fetch flags. | Covered | `removed.fetch_props_late_mode` | function f(PDOStatement $s) { $s->fetchAll(PDO::FETCH_ASSOC \| PDO::FETCH_PROPS_LATE); } is reported when the receiver's class is visible in the code. |
| PDO | A ValueError is now thrown if PDO::FETCH_INTO is used as a fetch mode in PDO::fetchAll(), similar to PDO::FETCH_LAZY. | Covered | `removed.fetch_all_into` | function f(PDOStatement $s) { $s->fetchAll(PDO::FETCH_INTO); } is reported when the receiver's class is visible in the code. |
| PDO_FIREBIRD | A ValueError is now thrown when trying to set a cursor name that is too long on a PDOStatement resulting from the Firebird driver. | Runtime only |  | Cursor name length depends on runtime values. |
| Session | Attempting to write session data where $_SESSION has a key containing the pipe character will now emit a warning instead of silently failing. | Covered | `behavior.session_key_pipe` | $_SESSION keys containing \| are reported. |
| SimpleXML | Passing an XPath expression that returns something other than a node set to SimpleXMLElement::xpath() will now emit a warning and return false, instead of silently failing and returning an empty… | Covered | `removed.xpath_non_node_set` | $xml = new SimpleXMLElement('&lt;a/>'); $xml->xpath('count(//a)') is reported when the receiver's class is visible in the code. |
| SPL | ArrayObject no longer accepts enums, as modifying the $name or $value properties can break engine assumptions. | Gap |  | `new ArrayObject(Suit::H)` with an enum case was tested and not reported. |
| SPL | SplFileObject::fwrite's parameter $length is now nullable. | Covered | `removed.builtin_override_signature` | SplFileObject::fwrite() overrides with a non-nullable $length type are reported. |
| Standard | Using a printf-family function with a formatter that did not specify the precision previously incorrectly reset the precision instead of treating it as a precision of 0. | Covered | `behavior.printf_bare_precision` | printf-family formats with a precision but no digits, such as %.f, are reported. |
| SOAP | SoapClient::__doRequest() expects a new, optional $uriParserClass parameter as described in https://wiki.php.net/rfc/url_parsing_api#plugability. | Covered | `removed.builtin_override_signature` | SoapClient::__doRequest() overrides with fewer than six parameters are reported. |
| Core | Trying to produce output (e.g. with `echo`) within a user output handler is deprecated. | Covered | `deprecated.output_in_ob_handler` | ob_start() with a closure that contains echo or print is reported. |
| Core | Non-canonical cast names (boolean), (integer), (double), and (binary) have been deprecated, use (bool), (int), (float), and (string) respectively. | Covered | `deprecated.non_canonical_cast` | (boolean)/(integer)/(double)/(binary) casts are reported. |
| Core | The $exclude_disabled parameter of the get_defined_functions() function has been deprecated, as it no longer has any effect since PHP 8.0. | Covered | `deprecated.get_defined_functions_arg` | get_defined_functions(true) is reported. |
| Core | Terminating case statements with a semicolon instead of a colon has been deprecated. | Covered | `deprecated.case_semicolon` | `case 1;` is reported. |
| Core | The backtick operator as an alias for shell_exec() has been deprecated. | Covered | `deprecated.backtick` | Backtick shell execution is reported. |
| Core | Returning null from __debugInfo() has been deprecated. | Covered | `deprecated.debug_info_null` | class A { function __debugInfo() { return null; } } is reported. |
| Core | The report_memleaks INI directive has been deprecated. | Covered | `deprecated.ini_setting` | report_memleaks in php.ini and ini_set() is reported. |
| Core | Constant redeclaration has been deprecated. | Covered | `deprecated.constant_redeclared` | Constants declared twice in the same file with const or unguarded top-level define() are reported. |
| Core | Enacted the follow-up phase of the "Path to Saner Increment/Decrement operators" RFC, meaning that incrementing non-numeric strings is now deprecated. | Gap |  | Incrementing a literal non-numeric string variable (`$s = 'abc'; $s++;`) was tested and not reported. |
| Core | The following closure binding issues, which already emit an E_WARNING, are now deprecated: | Covered | `deprecated.closure_bind` | Binding an object to a static closure, and unbinding $this from a closure that uses it, are reported for literal closures. |
| Core | Using null as an array offset or when calling array_key_exists() is now deprecated. | Covered | `deprecated.null_offset` | `$a[null]` and `array_key_exists(null, $a)` are reported. |
| Core | Deriving $_SERVER['argc'] and $_SERVER['argv'] from the query string for non-CLI SAPIs has been deprecated. | Runtime only |  | Whether argv comes from the query string depends on the SAPI at runtime. |
| Core | The __sleep() and __wakeup() magic methods have been soft-deprecated. | Not applicable |  | A soft deprecation only: PHP 8.5 raises nothing for __sleep() or __wakeup(), so there is nothing to change yet. |
| Curl | The curl_close() function has been deprecated, as CurlHandle objects are freed automatically. | Covered | `deprecated.curl_close` | curl_close() is reported. |
| Curl | The curl_share_close() function has been deprecated, as CurlShareHandle objects are freed automatically. | Covered | `deprecated.curl_share_close` | curl_share_close() is reported. |
| Date | The DATE_RFC7231 and DateTimeInterface::RFC7231 constants have been deprecated. | Covered | `deprecated.date_rfc7231` | DATE_RFC7231, DateTimeInterface::RFC7231 and DateTime::RFC7231 are reported. |
| Date | The __wakeup() magic method of DateTimeInterface, DateTime, DateTimeImmutable, DateTimeZone, DateInterval, and DatePeriod has been deprecated in favour of the __unserialize() magic method. | Covered | `deprecated.date_wakeup`<br>`deprecated.builtin_method` | Calling and overriding DateTime::__wakeup() are reported. |
| FileInfo | The finfo_close() function has been deprecated. | Covered | `deprecated.finfo_close` | finfo_close() is reported. |
| FileInfo | The $context parameter of the finfo_buffer() and finfo::buffer() functions has been deprecated as it is ignored. | Covered | `deprecated.finfo_buffer_context` | finfo_buffer($f, $b, FILEINFO_NONE, $ctx) is reported. |
| GD | The imagedestroy() function has been deprecated, as GdImage objects are freed automatically. | Covered | `deprecated.imagedestroy` | imagedestroy() is reported. |
| Hash | The MHASH_* constants have been deprecated. | Covered | `deprecated.mhash_constant` | MHASH_* constants are reported. |
| Intl | The intl.error_level INI setting has been deprecated. | Covered | `deprecated.ini_setting` | ini_set('intl.error_level', E_WARNING) is reported. |
| MySQLi | The mysqli_execute() alias function has been deprecated. | Covered | `deprecated.mysqli_execute` | mysqli_execute() is reported. |
| OpenSSL | The $key_length parameter for openssl_pkey_derive() has been deprecated. | Covered | `deprecated.openssl_pkey_derive_length` | openssl_pkey_derive($pub, $priv, 32) is reported. |
| PDO | The "uri:" DSN scheme has been deprecated due to security concerns with DSNs coming from remote URIs. | Covered | `deprecated.pdo_uri_dsn` | new PDO('uri:file:///etc/dsn') is reported. |
| PDO | Driver specific constants in the PDO class have been deprecated. | Covered | `deprecated.pdo_driver_constant` | PDO::MYSQL_ATTR_*/PGSQL_*/SQLITE_*/DBLIB_* constants are reported. |
| PDO | Driver specific methods in the PDO class have been deprecated. | Covered | `deprecated.pdo_driver_method` | $pdo = new PDO('sqlite::memory:'); $pdo->sqliteCreateFunction('f', 'g') is reported when the receiver's class is visible in the code. |
| Reflection | The setAccessible() methods of various Reflection objects have been deprecated, as those no longer have an effect. | Covered | `deprecated.set_accessible` | setAccessible() on ReflectionProperty and ReflectionMethod is reported. |
| Reflection | Calling ReflectionClass::getConstant() for constants that do not exist has been deprecated. | Runtime only |  | Whether a constant exists depends on the reflected class at runtime. |
| Reflection | Calling ReflectionProperty::getDefaultValue() for properties without default values has been deprecated. | Runtime only |  | Whether a property has a default depends on the reflected class at runtime. |
| SPL | Unregistering all autoloaders by passing the spl_autoload_call() function as a callback argument to spl_autoload_unregister() has been deprecated. | Covered | `deprecated.unregister_autoload_call` | spl_autoload_unregister('spl_autoload_call') is reported. |
| SPL | The SplObjectStorage::contains(), SplObjectStorage::attach(), and SplObjectStorage::detach() methods have been deprecated in favour of SplObjectStorage::offsetExists(),… | Covered | `deprecated.spl_object_storage` | SplObjectStorage attach()/contains()/detach() are reported. |
| SPL | Using ArrayObject and ArrayIterator with objects has been deprecated. | Covered | `deprecated.array_object_object` | new ArrayObject(new stdClass) is reported. |
| Standard | The socket_set_timeout() alias function has been deprecated. | Covered | `deprecated.socket_set_timeout` | socket_set_timeout() is reported. |
| Standard | Passing null to to readdir(), rewinddir(), and closedir() to use the last opened directory has been deprecated. | Covered | `deprecated.implicit_argument` | readdir(null) is reported. |
| Standard | Passing integers outside the interval [0, 255] to chr() is now deprecated. | Covered | `deprecated.chr_out_of_range` | chr(256) is reported. |
| Standard | Passing a string which is not a single byte to ord() is now deprecated, this is indicative of a bug. | Covered | `deprecated.ord_length` | ord('ab') is reported. |
| Standard | The locally predefined variable $http_response_header is deprecated. | Covered | `deprecated.http_response_header` | $http_response_header usage is reported. |
| XML | The xml_parser_free() function has been deprecated, as XMLParser objects are freed automatically. | Covered | `deprecated.xml_parser_free` | xml_parser_free() is reported. |
