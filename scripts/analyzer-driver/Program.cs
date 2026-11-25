// Usage: AnalyzerDriver <folder> <analyzer dll> <id prefixes> [--refs <folder>]... [--framework] [--implicit-usings] [--stats]. Parses every .cs file below the folder into
// one compilation (references: the core library, or without it every .dll below each --refs folder and, with --framework, the reference
// packs of the .NET SDK, Microsoft.NETCore.App and Microsoft.AspNetCore.App) and runs every analyzer of the DLL (in the
// `analyzer` folder of the output; its other DLLs are found next to it) with the rules it ships on (IsEnabledByDefault and a severity
// above Hidden, read from its descriptors) as warnings and the others suppressed; prints one JSON object per finding of an id that
// starts with one of the comma-separated prefixes: {"file", "rule", "line", "message"}, the file relative to the folder. The counts of
// rules on and off go to stderr; --implicit-usings adds the global usings of a project with ImplicitUsings on (the
// corpus has none of its project files); with --stats the wall time and peak memory.
using System.Collections.Immutable;
using System.Reflection;
using System.Text.Json;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.Diagnostics;

var root = Path.GetFullPath(args[0]);
var prefixes = args[2].Split(',');
var refFolders = args.Select((a, i) => (a, i)).Where(x => x.a == "--refs").Select(x => args[x.i + 1]).ToList();
if (args.Contains("--framework"))
{
    var packs = Path.GetFullPath(Path.Combine(System.Runtime.InteropServices.RuntimeEnvironment.GetRuntimeDirectory(), "..", "..", "..", "packs"));
    foreach (var pack in new[] { "Microsoft.NETCore.App.Ref", "Microsoft.AspNetCore.App.Ref" })
        refFolders.Add(Directory.GetDirectories(Path.Combine(packs, pack)).OrderBy(d => Version.TryParse(Path.GetFileName(d), out var v) ? v : new Version()).Last() + "/ref/net10.0");
}
var clock = System.Diagnostics.Stopwatch.StartNew();
var files = Directory.EnumerateFiles(root, "*.cs", SearchOption.AllDirectories).Order(StringComparer.Ordinal).ToList();
// Documentation comments are parsed and checked as in a build with GenerateDocumentationFile on (csc /doc: DocumentationMode.Diagnose);
// with it off StyleCop reports SA0001 and the `///` lines are plain comments (docs/ai/development/parity.md).
var parse = new CSharpParseOptions(LanguageVersion.Latest, DocumentationMode.Diagnose);
var trees = files.AsParallel().AsOrdered().Select(f => CSharpSyntaxTree.ParseText(File.ReadAllText(f), parse, f)).ToList();

var assembly = Assembly.LoadFrom(Path.Combine(AppContext.BaseDirectory, "analyzer", args[1]));
var analyzers = assembly.GetTypes()
    .Where(t => !t.IsAbstract && typeof(DiagnosticAnalyzer).IsAssignableFrom(t) && t.GetCustomAttributes<DiagnosticAnalyzerAttribute>().Any())
    .Select(t => (DiagnosticAnalyzer)Activator.CreateInstance(t)!)
    .ToImmutableArray();
var descriptors = analyzers.SelectMany(a => a.SupportedDiagnostics).GroupBy(d => d.Id).Select(g => g.First()).ToList();
var states = descriptors.ToDictionary(d => d.Id, d => d.IsEnabledByDefault && d.DefaultSeverity > DiagnosticSeverity.Hidden ? ReportDiagnostic.Warn : ReportDiagnostic.Suppress);
Console.Error.WriteLine($"rules on by default: {states.Count(s => s.Value == ReportDiagnostic.Warn)}, off or hidden: {states.Count(s => s.Value == ReportDiagnostic.Suppress)}");
var options = new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary).WithSpecificDiagnosticOptions(states);
var references = (refFolders.Count == 0 ? new[] { typeof(object).Assembly.Location } : [])
    .Concat(refFolders.SelectMany(d => Directory.EnumerateFiles(d, "*.dll", SearchOption.AllDirectories))).Distinct()
    .Select(f => (MetadataReference)MetadataReference.CreateFromFile(f)).ToList();
const string implicitUsings = "implicit-usings";
if (args.Contains("--implicit-usings"))
    trees.Add(CSharpSyntaxTree.ParseText(string.Concat(new[] { "System", "System.Collections.Generic", "System.IO", "System.Linq", "System.Net.Http", "System.Threading", "System.Threading.Tasks" }.Select(n => $"global using global::{n};\n")), parse, implicitUsings));
var compilation = CSharpCompilation.Create("corpus", trees, references, options);
var driver = compilation.WithAnalyzers(analyzers, new CompilationWithAnalyzersOptions(
    new AnalyzerOptions(ImmutableArray<AdditionalText>.Empty), null, concurrentAnalysis: true, logAnalyzerExecutionTime: false));
var found = driver.GetAnalyzerDiagnosticsAsync().GetAwaiter().GetResult();
var output = Console.Out;
foreach (var d in found.OrderBy(d => d.Location.SourceTree?.FilePath, StringComparer.Ordinal).ThenBy(d => d.Location.SourceSpan.Start))
{
    if (!d.Location.IsInSource || d.Location.SourceTree!.FilePath == implicitUsings || !prefixes.Any(p => d.Id.StartsWith(p, StringComparison.Ordinal))) continue;
    if (d.Severity == DiagnosticSeverity.Hidden || states.GetValueOrDefault(d.Id) != ReportDiagnostic.Warn) continue;
    var span = d.Location.GetLineSpan();
    output.WriteLine(JsonSerializer.Serialize(new
    {
        file = Path.GetRelativePath(root, d.Location.SourceTree!.FilePath).Replace('\\', '/'),
        rule = d.Id,
        line = span.StartLinePosition.Line + 1,
        message = d.GetMessage(),
    }));
}
if (args.Contains("--stats"))
    Console.Error.WriteLine($"wall {clock.Elapsed.TotalSeconds:F0} s, peak {System.Diagnostics.Process.GetCurrentProcess().PeakWorkingSet64 >> 20} MB");
