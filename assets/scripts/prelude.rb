class Array

  # Enumerable#sum
  def sum(init = 0)
    accum = init
    if block_given?
      each do |x|
        accum += yield(x)
      end
    else
      each do |x|
        accum += x
      end
    end

    accum
  end

end

class Handlers
  
  def initialize
    # ruruby doesn't seem to support Hash.new with a block parameter.
    # @handlers = Hash.new {|h,k| h[k] = [] }
    @handlers = {}
  end

  def add(event_name, &block)
    event_name = event_name.to_sym
    hs = @handlers[event_name] ||= []
    hs << block
  end

  def trigger(context, event_name, *args)
    (@handlers[event_name.to_sym] || []).each do |block|
      # ruruby doesn't like both kwargs and block.
      # context.instance_exec(*args, **kwargs, &block)
      context.instance_exec(*args, &block)
    end
  end
  
end

# To use this, you must define @handlers = Handlers.new
module Eventable

  def trigger(event_name, *args)
    if defined?(@handlers)
      @handlers.trigger(self, event_name, *args)
    end
    class_handlers = self.class.instance_variable_get(:@handlers)
    class_handlers.trigger(self, event_name, *args)
  end

end

module GameMethods
  def players; game.players; end
end

class BasePlayer
  # extend Defineable
  include Eventable
  include GameMethods

  @handlers = Handlers.new

  class << self

    def on_collect(object_name, &block)
      object_name = object_name.to_sym
      if object_name == :any
        on(:collect, &block)
      else
        on(:collect) do |object|
          if object.name.to_sym == object_name
            instance_exec(object, &block)
          end
        end
      end
    end
    
  end

  attr_reader :id
  attr_reader :game

  def initialize(id:, game:)
    @id = id
    @game = game
    run_setup
  end

  def map(); game.map; end

end

class MapObject
  attr_reader :name

  def initialize(name:)
    @name = name
  end
end

class BaseMap
  # extend Defineable
  include Eventable
  include GameMethods

  @handlers = Handlers.new

  class << self
    def on_enter(&block); on(:enter, &block); end
    def on_exit(&block);  on(:exit,  &block); end
    def on_load(&block);  on(:load,  &block); end
  end

  attr_reader :id
  attr_reader :filename
  attr_reader :game

  def initialize(id:, filename:, game:)
    @id = id
    @filename = filename
    @game = game
    run_setup
  end

  def show(object_name)
    update_object(object_name, visible: true)
  end

  def make_collectable(object_name)
    update_object(object_name, collectable: true)
  end

  def update_object(object_name, **kwargs)
    ScriptCore.update_map_objects_by_name(@id, object_name, kwargs)
  end

end

class BaseGame
  # extend Defineable
  include Eventable

  class << self
    # TODO: load, quit
    def on_new_game(&block); on(:new_game, &block); end
  end

  attr_reader :map
  attr_reader :players

  def initialize()
    @map = nil
    @maps = {}
    @players = []
    run_setup
  end

  def trigger_new_game(player_ids)
    @players = player_ids.map {|id| Player.new(id: id, game: self) }
    trigger(:new_game)
  end

  def trigger_enter_map(map)
    @map.trigger(:exit) if @map
    @map = map
    @map.trigger(:enter) if @map
  end

  def player_by_id(id)
    @players.find {|pl| pl.id == id }
  end

  def player_by_id!(id)
    player = player_by_id(id)
    raise "player not found: id=#{id.inspect}" unless player

    player
  end

  def find_or_create_map(id:, filename:)
    map = @maps[id]
    unless map
      map = Map.new(id: id, filename: filename, game: self)
      @maps[id] = map
    end

    map
  end

  def dialogue(node_name)
    ScriptCore.start_dialogue(node_name.to_s)
  end

end

# To use this, you must define @handlers = Handlers.new
# module Defineable
#
# ruruby doesn't support extend, so instead of defining the module and calling
# `extend Defineable`, we manually define here.
[BasePlayer, BaseMap, BaseGame].each do |klass|

  class << klass

    def define(*names)
      attr_accessor *names
    end

    def setup(&block)
      @setup_blocks ||= []
      @setup_blocks << block
    end

    def setup_blocks
      @setup_blocks || []
    end

    def on(event_name, &block)
      @handlers ||= Handlers.new
      @handlers.add(event_name, &block)
    end

  end

  def run_setup
    (self.class.setup_blocks || []).each do |block|
      self.instance_exec(&block)
    end
  end

end

class Game < BaseGame; end
class Player < BasePlayer; end
class Map < BaseMap; end

# ruruby doesn't support include at the top level or extend.  Otherwise, these
# would be defined in a DSL module.
def game(&block)
  Game.class_eval(&block)
end

def player(&block)
  Player.class_eval(&block)
end

def map(&block)
  Map.class_eval(&block)
end
