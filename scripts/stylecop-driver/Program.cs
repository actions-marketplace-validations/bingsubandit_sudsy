// Usage: StyleCopDriver <folder>. Parses every .cs file below the folder into one compilation (no references but the core
// library) and runs every StyleCop analyzer over it with the rules the analyzer ships on (IsEnabledByDefault and a severity
// above Hidden, read from its descriptors) as warnings and the others suppressed; prints one JSON object per finding:
// {"file", "rule", "line", "message"}, the file relative to the folder. The counts of rules on and off go to stderr.
using System.Collections.Immutable;
using System.Reflection;
using System.Text.Json;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.Diagnostics;

var root = Path.GetFullPath(args[0]);
var files = Directory.EnumerateFiles(root, "*.cs", SearchOption.AllDirectories).Order(StringComparer.Ordinal).ToList();
// Documentation comments are parsed and checked as in a build with GenerateDocumentationFile on (csc /doc: DocumentationMode.Diagnose);
// with it off StyleCop reports SA0001 and the `///` lines are plain comments (docs/ai/development/parity.md).
var parse = new CSharpParseOptions(LanguageVersion.Latest, DocumentationMode.Diagnose);
var trees = files.AsParallel().AsOrdered().Select(f => CSharpSyntaxTree.ParseText(File.ReadAllText(f), parse, f)).ToList();

var assembly = Assembly.LoadFrom(Path.Combine(AppContext.BaseDirectory, "StyleCop.Analyzers.dll"));
var analyzers = assembly.GetTypes()
    .Where(t => !t.IsAbstract && typeof(DiagnosticAnalyzer).IsAssignableFrom(t) && t.GetCustomAttributes<DiagnosticAnalyzerAttribute>().Any())
    .Select(t => (DiagnosticAnalyzer)Activator.CreateInstance(t)!)
    .ToImmutableArray();
var descriptors = analyzers.SelectMany(a => a.SupportedDiagnostics).GroupBy(d => d.Id).Select(g => g.First()).ToList();
var states = descriptors.ToDictionary(d => d.Id, d => d.IsEnabledByDefault && d.DefaultSeverity > DiagnosticSeverity.Hidden ? ReportDiagnostic.Warn : ReportDiagnostic.Suppress);
Console.Error.WriteLine($"stylecop rules on by default: {states.Count(s => s.Value == ReportDiagnostic.Warn)}, off or hidden: {states.Count(s => s.Value == ReportDiagnostic.Suppress)}");
var options = new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary).WithSpecificDiagnosticOptions(states);
var compilation = CSharpCompilation.Create("corpus", trees, [MetadataReference.CreateFromFile(typeof(object).Assembly.Location)], options);
var driver = compilation.WithAnalyzers(analyzers, new CompilationWithAnalyzersOptions(
    new AnalyzerOptions(ImmutableArray<AdditionalText>.Empty), null, concurrentAnalysis: true, logAnalyzerExecutionTime: false));
var found = driver.GetAnalyzerDiagnosticsAsync().GetAwaiter().GetResult();
var output = Console.Out;
foreach (var d in found.OrderBy(d => d.Location.SourceTree?.FilePath, StringComparer.Ordinal).ThenBy(d => d.Location.SourceSpan.Start))
{
    if (!d.Location.IsInSource || !(d.Id.StartsWith("SA") || d.Id.StartsWith("SX"))) continue;
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
